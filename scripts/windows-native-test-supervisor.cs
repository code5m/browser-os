// Definitions only. PowerShell loads this file explicitly during an authorized
// native run. No shell, PID enumeration or post-start Job assignment is used.
// Windows 10+/Server 2016+: PROC_THREAD_ATTRIBUTE_JOB_LIST is mandatory.
using System;
using System.ComponentModel;
using System.Diagnostics;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;
using Microsoft.Win32.SafeHandles;

namespace BrowserOs.NativeTests
{
    public sealed class SupervisionResult
    {
        public bool ProcessCreated { get; internal set; }
        public bool AtomicJobMembershipVerified { get; internal set; }
        public bool RootExited { get; internal set; }
        public uint? ExitCode { get; internal set; }
        public bool TimedOut { get; internal set; }
        public bool OutputLimitExceeded { get; internal set; }
        public bool DescendantsOutlivedRoot { get; internal set; }
        public bool TerminationRequested { get; internal set; }
        public bool JobQuerySucceeded { get; internal set; }
        public bool JobEmpty { get; internal set; }
        public uint? ActiveProcesses { get; internal set; }
        public bool CleanupComplete { get; internal set; }
        public string FailureCode { get; internal set; }
        public int NativeError { get; internal set; }
        public long ElapsedMilliseconds { get; internal set; }
    }

    public static class Supervisor
    {
        private const uint CREATE_SUSPENDED = 0x00000004;
        private const uint CREATE_UNICODE_ENVIRONMENT = 0x00000400;
        private const uint EXTENDED_STARTUPINFO_PRESENT = 0x00080000;
        private const uint CREATE_NO_WINDOW = 0x08000000;
        private const uint STARTF_USESTDHANDLES = 0x00000100;
        private const uint JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE = 0x00002000;
        private const uint WAIT_OBJECT_0 = 0;
        private const uint WAIT_TIMEOUT = 258;
        private const uint WAIT_FAILED = 0xffffffff;
        private const uint GENERIC_WRITE = 0x40000000;
        private const uint FILE_READ_ATTRIBUTES = 0x80;
        private const uint FILE_SHARE_READ = 1;
        private const uint CREATE_ALWAYS = 2;
        private const uint FILE_ATTRIBUTE_NORMAL = 0x80;
        private static readonly IntPtr HANDLE_LIST = new IntPtr(0x00020002);
        private static readonly IntPtr JOB_LIST = new IntPtr(0x0002000d);

        private sealed class OwnedHandle : SafeHandleZeroOrMinusOneIsInvalid
        {
            internal OwnedHandle(IntPtr value) : base(true) { SetHandle(value); }
            protected override bool ReleaseHandle() { return CloseHandle(handle); }
        }

        [StructLayout(LayoutKind.Sequential)]
        private struct SECURITY_ATTRIBUTES
        {
            internal int Length;
            internal IntPtr SecurityDescriptor;
            [MarshalAs(UnmanagedType.Bool)] internal bool InheritHandle;
        }

        [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
        private struct STARTUPINFO
        {
            internal uint cb;
            internal IntPtr lpReserved, lpDesktop, lpTitle;
            internal uint dwX, dwY, dwXSize, dwYSize, dwXCountChars, dwYCountChars;
            internal uint dwFillAttribute, dwFlags;
            internal ushort wShowWindow, cbReserved2;
            internal IntPtr lpReserved2, hStdInput, hStdOutput, hStdError;
        }

        [StructLayout(LayoutKind.Sequential)]
        private struct STARTUPINFOEX
        {
            internal STARTUPINFO StartupInfo;
            internal IntPtr AttributeList;
        }

        [StructLayout(LayoutKind.Sequential)]
        private struct PROCESS_INFORMATION
        {
            internal IntPtr Process, Thread;
            internal uint ProcessId, ThreadId;
        }

        [StructLayout(LayoutKind.Sequential)]
        private struct BASIC_LIMIT_INFORMATION
        {
            internal long PerProcessUserTimeLimit, PerJobUserTimeLimit;
            internal uint LimitFlags;
            internal UIntPtr MinimumWorkingSetSize, MaximumWorkingSetSize;
            internal uint ActiveProcessLimit;
            internal UIntPtr Affinity;
            internal uint PriorityClass, SchedulingClass;
        }

        [StructLayout(LayoutKind.Sequential)]
        private struct IO_COUNTERS
        {
            internal ulong ReadOperationCount, WriteOperationCount, OtherOperationCount;
            internal ulong ReadTransferCount, WriteTransferCount, OtherTransferCount;
        }

        [StructLayout(LayoutKind.Sequential)]
        private struct EXTENDED_LIMIT_INFORMATION
        {
            internal BASIC_LIMIT_INFORMATION BasicLimitInformation;
            internal IO_COUNTERS IoInfo;
            internal UIntPtr ProcessMemoryLimit, JobMemoryLimit, PeakProcessMemoryUsed, PeakJobMemoryUsed;
        }

        [StructLayout(LayoutKind.Sequential)]
        private struct BASIC_ACCOUNTING_INFORMATION
        {
            internal long TotalUserTime, TotalKernelTime, ThisPeriodTotalUserTime, ThisPeriodTotalKernelTime;
            internal uint TotalPageFaultCount, TotalProcesses, ActiveProcesses, TotalTerminatedProcesses;
        }

        [DllImport("kernel32.dll", SetLastError = true)] [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool CloseHandle(IntPtr handle);
        [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        private static extern IntPtr CreateJobObjectW(IntPtr attributes, string name);
        [DllImport("kernel32.dll", SetLastError = true)] [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool SetInformationJobObject(IntPtr job, int informationClass,
            ref EXTENDED_LIMIT_INFORMATION information, uint length);
        [DllImport("kernel32.dll", SetLastError = true)] [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool QueryInformationJobObject(IntPtr job, int informationClass,
            out BASIC_ACCOUNTING_INFORMATION information, uint length, IntPtr returnedLength);
        [DllImport("kernel32.dll", SetLastError = true)] [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool IsProcessInJob(IntPtr process, IntPtr job, [MarshalAs(UnmanagedType.Bool)] out bool member);
        [DllImport("kernel32.dll", SetLastError = true)] [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool TerminateJobObject(IntPtr job, uint exitCode);
        [DllImport("kernel32.dll", SetLastError = true)] [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool TerminateProcess(IntPtr process, uint exitCode);
        [DllImport("kernel32.dll", SetLastError = true)]
        private static extern uint WaitForSingleObject(IntPtr handle, uint milliseconds);
        [DllImport("kernel32.dll", SetLastError = true)] [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool GetExitCodeProcess(IntPtr process, out uint exitCode);
        [DllImport("kernel32.dll", SetLastError = true)]
        private static extern uint ResumeThread(IntPtr thread);
        [DllImport("kernel32.dll", SetLastError = true)] [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool InitializeProcThreadAttributeList(IntPtr list, int count, int flags, ref UIntPtr size);
        [DllImport("kernel32.dll", SetLastError = true)] [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool UpdateProcThreadAttribute(IntPtr list, uint flags, IntPtr attribute,
            IntPtr value, UIntPtr size, IntPtr previous, IntPtr returned);
        [DllImport("kernel32.dll")]
        private static extern void DeleteProcThreadAttributeList(IntPtr list);
        [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)] [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool CreateProcessW(string application, StringBuilder commandLine,
            IntPtr processAttributes, IntPtr threadAttributes, [MarshalAs(UnmanagedType.Bool)] bool inheritHandles,
            uint creationFlags, IntPtr environment, string currentDirectory,
            ref STARTUPINFOEX startup, out PROCESS_INFORMATION information);
        [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        private static extern IntPtr CreateFileW(string path, uint access, uint share,
            ref SECURITY_ATTRIBUTES attributes, uint disposition, uint flags, IntPtr template);
        [DllImport("kernel32.dll", SetLastError = true)] [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool CreatePipe(out IntPtr read, out IntPtr write,
            ref SECURITY_ATTRIBUTES attributes, uint size);
        [DllImport("kernel32.dll", SetLastError = true)] [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool GetFileSizeEx(IntPtr file, out long size);

        private static Win32Exception NativeFailure(string stage)
        {
            return new Win32Exception(Marshal.GetLastWin32Error(), stage);
        }

        private static void RememberFailure(SupervisionResult result, string stage, int error)
        {
            if (String.IsNullOrEmpty(result.FailureCode))
            {
                result.FailureCode = stage;
                result.NativeError = error;
            }
        }

        // The application path is passed separately to CreateProcessW. This
        // quotes each argv item using Windows CRT rules; no PowerShell/cmd layer.
        public static string QuoteArgument(string argument)
        {
            if (argument == null || argument.IndexOf('\0') >= 0)
                throw new ArgumentException("NATIVE_ARGUMENT_INVALID");
            StringBuilder quoted = new StringBuilder("\"");
            int slashes = 0;
            foreach (char value in argument)
            {
                if (value == '\\') { slashes++; continue; }
                if (value == '"')
                {
                    quoted.Append('\\', slashes * 2 + 1);
                    quoted.Append('"');
                    slashes = 0;
                    continue;
                }
                quoted.Append('\\', slashes);
                slashes = 0;
                quoted.Append(value);
            }
            quoted.Append('\\', slashes * 2);
            quoted.Append('"');
            return quoted.ToString();
        }

        private static bool ObserveJob(OwnedHandle job, SupervisionResult result)
        {
            BASIC_ACCOUNTING_INFORMATION accounting;
            if (!QueryInformationJobObject(job.DangerousGetHandle(), 1, out accounting,
                (uint)Marshal.SizeOf(typeof(BASIC_ACCOUNTING_INFORMATION)), IntPtr.Zero))
            {
                result.JobQuerySucceeded = false;
                result.JobEmpty = false;
                result.ActiveProcesses = null;
                RememberFailure(result, "NATIVE_JOB_ACCOUNTING_FAILED", Marshal.GetLastWin32Error());
                return false;
            }
            result.JobQuerySucceeded = true;
            result.ActiveProcesses = accounting.ActiveProcesses;
            result.JobEmpty = accounting.ActiveProcesses == 0;
            return true;
        }

        private static bool ObserveRoot(OwnedHandle process, SupervisionResult result)
        {
            uint wait = WaitForSingleObject(process.DangerousGetHandle(), 0);
            if (wait == WAIT_TIMEOUT) { return true; }
            if (wait != WAIT_OBJECT_0)
            {
                RememberFailure(result, "NATIVE_ROOT_WAIT_FAILED", Marshal.GetLastWin32Error());
                return false;
            }
            result.RootExited = true;
            uint code;
            if (!GetExitCodeProcess(process.DangerousGetHandle(), out code))
            {
                RememberFailure(result, "NATIVE_EXIT_CODE_FAILED", Marshal.GetLastWin32Error());
                return false;
            }
            result.ExitCode = code; // 259 is a real exit code once the handle is signaled.
            return true;
        }

        private static long OutputLength(OwnedHandle stdout, OwnedHandle stderr)
        {
            long outSize, errSize;
            if (!GetFileSizeEx(stdout.DangerousGetHandle(), out outSize)
                || !GetFileSizeEx(stderr.DangerousGetHandle(), out errSize))
                throw NativeFailure("NATIVE_OUTPUT_SIZE_FAILED");
            return checked(outSize + errSize);
        }

        public static SupervisionResult Run(string executable, string[] arguments,
            string stdoutPath, string stderrPath, string workingDirectory, int timeoutMilliseconds,
            int cleanupMilliseconds, long maximumOutputBytes)
        {
            if (String.IsNullOrWhiteSpace(executable) || !Path.IsPathRooted(executable)
                || executable.IndexOf('\0') >= 0 || arguments == null
                || !Path.IsPathRooted(stdoutPath) || !Path.IsPathRooted(stderrPath)
                || String.IsNullOrWhiteSpace(workingDirectory) || !Path.IsPathRooted(workingDirectory)
                || String.Equals(stdoutPath, stderrPath, StringComparison.OrdinalIgnoreCase)
                || timeoutMilliseconds < 1 || cleanupMilliseconds < 1 || maximumOutputBytes < 1)
                throw new ArgumentException("NATIVE_SUPERVISOR_ARGUMENT_INVALID");

            StringBuilder command = new StringBuilder(QuoteArgument(executable));
            foreach (string argument in arguments) command.Append(' ').Append(QuoteArgument(argument));
            if (command.Length >= 32767) throw new ArgumentException("NATIVE_COMMAND_LINE_TOO_LONG");

            SupervisionResult result = new SupervisionResult { FailureCode = "", CleanupComplete = false };
            Stopwatch elapsed = Stopwatch.StartNew();
            OwnedHandle job = null, process = null, primary = null, stdin = null, stdout = null, stderr = null;
            IntPtr attributes = IntPtr.Zero, jobValue = IntPtr.Zero, handleValues = IntPtr.Zero;
            bool attributesInitialized = false;
            try
            {
                job = new OwnedHandle(CreateJobObjectW(IntPtr.Zero, null));
                if (job.IsInvalid) throw NativeFailure("NATIVE_JOB_CREATE_FAILED");
                EXTENDED_LIMIT_INFORMATION limits = new EXTENDED_LIMIT_INFORMATION();
                limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                if (!SetInformationJobObject(job.DangerousGetHandle(), 9, ref limits,
                    (uint)Marshal.SizeOf(typeof(EXTENDED_LIMIT_INFORMATION))))
                    throw NativeFailure("NATIVE_JOB_LIMIT_FAILED");

                SECURITY_ATTRIBUTES inheritable = new SECURITY_ATTRIBUTES {
                    Length = Marshal.SizeOf(typeof(SECURITY_ATTRIBUTES)), InheritHandle = true
                };
                stdout = new OwnedHandle(CreateFileW(stdoutPath, GENERIC_WRITE | FILE_READ_ATTRIBUTES,
                    FILE_SHARE_READ, ref inheritable, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, IntPtr.Zero));
                if (stdout.IsInvalid) throw NativeFailure("NATIVE_STDOUT_OPEN_FAILED");
                stderr = new OwnedHandle(CreateFileW(stderrPath, GENERIC_WRITE | FILE_READ_ATTRIBUTES,
                    FILE_SHARE_READ, ref inheritable, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, IntPtr.Zero));
                if (stderr.IsInvalid) throw NativeFailure("NATIVE_STDERR_OPEN_FAILED");
                IntPtr inputRead, inputWrite;
                if (!CreatePipe(out inputRead, out inputWrite, ref inheritable, 0))
                    throw NativeFailure("NATIVE_STDIN_PIPE_FAILED");
                stdin = new OwnedHandle(inputRead);
                using (OwnedHandle inputEof = new OwnedHandle(inputWrite)) { }

                UIntPtr bytes = UIntPtr.Zero;
                InitializeProcThreadAttributeList(IntPtr.Zero, 2, 0, ref bytes);
                if (bytes == UIntPtr.Zero || bytes.ToUInt64() > 1024 * 1024)
                    throw new InvalidOperationException("NATIVE_ATTRIBUTE_SIZE_INVALID");
                attributes = Marshal.AllocHGlobal(checked((int)bytes.ToUInt64()));
                if (!InitializeProcThreadAttributeList(attributes, 2, 0, ref bytes))
                    throw NativeFailure("NATIVE_ATTRIBUTE_INIT_FAILED");
                attributesInitialized = true;
                jobValue = Marshal.AllocHGlobal(IntPtr.Size);
                Marshal.WriteIntPtr(jobValue, job.DangerousGetHandle());
                if (!UpdateProcThreadAttribute(attributes, 0, JOB_LIST, jobValue,
                    new UIntPtr((uint)IntPtr.Size), IntPtr.Zero, IntPtr.Zero))
                    throw NativeFailure("NATIVE_ATOMIC_JOB_ATTRIBUTE_FAILED");
                handleValues = Marshal.AllocHGlobal(IntPtr.Size * 3);
                Marshal.WriteIntPtr(handleValues, 0, stdin.DangerousGetHandle());
                Marshal.WriteIntPtr(handleValues, IntPtr.Size, stdout.DangerousGetHandle());
                Marshal.WriteIntPtr(handleValues, IntPtr.Size * 2, stderr.DangerousGetHandle());
                if (!UpdateProcThreadAttribute(attributes, 0, HANDLE_LIST, handleValues,
                    new UIntPtr((uint)(IntPtr.Size * 3)), IntPtr.Zero, IntPtr.Zero))
                    throw NativeFailure("NATIVE_HANDLE_ATTRIBUTE_FAILED");

                STARTUPINFOEX startup = new STARTUPINFOEX();
                startup.StartupInfo.cb = (uint)Marshal.SizeOf(typeof(STARTUPINFOEX));
                startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
                startup.StartupInfo.hStdInput = stdin.DangerousGetHandle();
                startup.StartupInfo.hStdOutput = stdout.DangerousGetHandle();
                startup.StartupInfo.hStdError = stderr.DangerousGetHandle();
                startup.AttributeList = attributes;
                PROCESS_INFORMATION information;
                if (!CreateProcessW(executable, command, IntPtr.Zero, IntPtr.Zero, true,
                    CREATE_SUSPENDED | CREATE_UNICODE_ENVIRONMENT | EXTENDED_STARTUPINFO_PRESENT | CREATE_NO_WINDOW,
                    IntPtr.Zero, workingDirectory, ref startup, out information))
                    throw NativeFailure("NATIVE_PROCESS_CREATE_FAILED");
                process = new OwnedHandle(information.Process);
                primary = new OwnedHandle(information.Thread);
                result.ProcessCreated = true;
                bool member;
                if (!IsProcessInJob(process.DangerousGetHandle(), job.DangerousGetHandle(), out member) || !member)
                    throw NativeFailure("NATIVE_ATOMIC_JOB_MEMBERSHIP_FAILED");
                result.AtomicJobMembershipVerified = true;
                if (ResumeThread(primary.DangerousGetHandle()) != 1)
                    throw NativeFailure("NATIVE_PRIMARY_RESUME_FAILED");
                primary.Dispose(); primary = null;

                long rootExitAt = -1;
                while (true)
                {
                    if (!ObserveRoot(process, result) || !ObserveJob(job, result)) break;
                    if (OutputLength(stdout, stderr) > maximumOutputBytes)
                    {
                        result.OutputLimitExceeded = true;
                        RememberFailure(result, "NATIVE_OUTPUT_LIMIT", 0);
                        break;
                    }
                    // A late observation cannot turn an expired startup/run
                    // budget into PASS merely because the root has now exited.
                    if (elapsed.ElapsedMilliseconds >= timeoutMilliseconds)
                    {
                        result.TimedOut = true;
                        RememberFailure(result, "NATIVE_TEST_TIMEOUT", 0);
                        break;
                    }
                    if (result.RootExited)
                    {
                        if (result.JobEmpty) break;
                        if (rootExitAt < 0) rootExitAt = elapsed.ElapsedMilliseconds;
                        if (elapsed.ElapsedMilliseconds - rootExitAt >= 1000)
                        {
                            result.DescendantsOutlivedRoot = true;
                            RememberFailure(result, "NATIVE_DESCENDANTS_OUTLIVED_ROOT", 0);
                            break;
                        }
                    }
                    Thread.Sleep(20);
                }
            }
            catch (Win32Exception error) { RememberFailure(result, error.Message, error.NativeErrorCode); }
            catch (Exception error) { RememberFailure(result, "NATIVE_SUPERVISOR_EXCEPTION:" + error.GetType().Name, 0); }
            finally
            {
                if (attributesInitialized) DeleteProcThreadAttributeList(attributes);
                if (attributes != IntPtr.Zero) Marshal.FreeHGlobal(attributes);
                if (jobValue != IntPtr.Zero) Marshal.FreeHGlobal(jobValue);
                if (handleValues != IntPtr.Zero) Marshal.FreeHGlobal(handleValues);

                if (job != null && !job.IsInvalid)
                {
                    bool queried = ObserveJob(job, result);
                    if (!queried || !result.JobEmpty)
                    {
                        result.TerminationRequested = true;
                        if (!TerminateJobObject(job.DangerousGetHandle(), 1))
                            RememberFailure(result, "NATIVE_JOB_TERMINATE_FAILED", Marshal.GetLastWin32Error());
                    }
                    // A membership failure is handled while the primary thread
                    // is still suspended; terminate that exact handle as well.
                    if (process != null && !result.AtomicJobMembershipVerified)
                        TerminateProcess(process.DangerousGetHandle(), 1);
                    Stopwatch cleanup = Stopwatch.StartNew();
                    do
                    {
                        bool rootObserved = process == null || ObserveRoot(process, result);
                        bool jobObserved = ObserveJob(job, result);
                        if (rootObserved && jobObserved && result.JobEmpty
                            && (!result.ProcessCreated || result.RootExited))
                        {
                            result.CleanupComplete = true;
                            break;
                        }
                        Thread.Sleep(20);
                    } while (cleanup.ElapsedMilliseconds < cleanupMilliseconds);
                    if (!result.CleanupComplete) RememberFailure(result, "NATIVE_JOB_CLEANUP_UNCONFIRMED", 0);
                }
                else { result.CleanupComplete = !result.ProcessCreated; }

                // KILL_ON_JOB_CLOSE remains the final fallback. It does not
                // turn an unconfirmed tree into CleanupComplete=true.
                if (primary != null) primary.Dispose();
                if (process != null) process.Dispose();
                if (stdin != null) stdin.Dispose();
                if (stdout != null) stdout.Dispose();
                if (stderr != null) stderr.Dispose();
                if (job != null) job.Dispose();
                result.ElapsedMilliseconds = elapsed.ElapsedMilliseconds;
            }
            return result;
        }
    }
}
