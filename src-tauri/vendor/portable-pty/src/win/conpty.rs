use crate::cmdbuilder::CommandBuilder;
use crate::win::psuedocon::PsuedoCon;
use crate::{Child, MasterPty, PtyPair, PtySize, PtySystem, SlavePty};
use anyhow::{ensure, Error};
use filedescriptor::{FileDescriptor, Pipe};
use std::sync::{Arc, Mutex};
use winapi::um::wincon::COORD;

fn validated_dimensions(rows: u16, cols: u16) -> anyhow::Result<COORD> {
    ensure!(
        rows > 0 && cols > 0 && rows <= i16::MAX as u16 && cols <= i16::MAX as u16,
        "ConPTY size outside supported 1..=32767 rows/cols"
    );
    Ok(COORD {
        X: cols as i16,
        Y: rows as i16,
    })
}

#[derive(Default)]
pub struct ConPtySystem {}

impl PtySystem for ConPtySystem {
    fn openpty(&self, size: PtySize) -> anyhow::Result<PtyPair> {
        // COORD uses signed 16-bit values. Truncating an arbitrary u16 can
        // turn a large requested terminal into a negative or zero console.
        let coordinates = validated_dimensions(size.rows, size.cols)?;
        let stdin = Pipe::new()?;
        let stdout = Pipe::new()?;

        let con = PsuedoCon::new(coordinates, stdin.read, stdout.write)?;

        let master = ConPtyMasterPty {
            inner: Arc::new(Mutex::new(Inner {
                con,
                readable: stdout.read,
                writable: Some(stdin.write),
                size,
            })),
        };

        let slave = ConPtySlavePty {
            inner: master.inner.clone(),
        };

        Ok(PtyPair {
            master: Box::new(master),
            slave: Box::new(slave),
        })
    }
}

struct Inner {
    // Drop communication endpoints before ClosePseudoConsole. On older
    // Windows builds closing a pseudo console with an undrained output pipe
    // can otherwise block its destructor indefinitely.
    readable: FileDescriptor,
    writable: Option<FileDescriptor>,
    con: PsuedoCon,
    size: PtySize,
}

impl Inner {
    pub fn resize(
        &mut self,
        num_rows: u16,
        num_cols: u16,
        pixel_width: u16,
        pixel_height: u16,
    ) -> Result<(), Error> {
        self.con.resize(validated_dimensions(num_rows, num_cols)?)?;
        self.size = PtySize {
            rows: num_rows,
            cols: num_cols,
            pixel_width,
            pixel_height,
        };
        Ok(())
    }
}

#[derive(Clone)]
pub struct ConPtyMasterPty {
    inner: Arc<Mutex<Inner>>,
}

pub struct ConPtySlavePty {
    inner: Arc<Mutex<Inner>>,
}

impl MasterPty for ConPtyMasterPty {
    fn resize(&self, size: PtySize) -> anyhow::Result<()> {
        let mut inner = self.inner.lock().unwrap();
        inner.resize(size.rows, size.cols, size.pixel_width, size.pixel_height)
    }

    fn get_size(&self) -> Result<PtySize, Error> {
        let inner = self.inner.lock().unwrap();
        Ok(inner.size.clone())
    }

    fn try_clone_reader(&self) -> anyhow::Result<Box<dyn std::io::Read + Send>> {
        Ok(Box::new(self.inner.lock().unwrap().readable.try_clone()?))
    }

    fn take_writer(&self) -> anyhow::Result<Box<dyn std::io::Write + Send>> {
        Ok(Box::new(
            self.inner
                .lock()
                .unwrap()
                .writable
                .take()
                .ok_or_else(|| anyhow::anyhow!("writer already taken"))?,
        ))
    }
}

impl SlavePty for ConPtySlavePty {
    fn spawn_command_in_job(
        &self,
        cmd: CommandBuilder,
        job: std::os::windows::io::RawHandle,
    ) -> anyhow::Result<Box<dyn Child + Send + Sync>> {
        let inner = self.inner.lock().unwrap();
        let child = inner.con.spawn_command_in_job(cmd, job as _)?;
        Ok(Box::new(child))
    }

    fn spawn_command(&self, cmd: CommandBuilder) -> anyhow::Result<Box<dyn Child + Send + Sync>> {
        let inner = self.inner.lock().unwrap();
        let child = inner.con.spawn_command(cmd)?;
        Ok(Box::new(child))
    }

    fn spawn_command_suspended(
        &self,
        cmd: CommandBuilder,
    ) -> anyhow::Result<Box<dyn Child + Send + Sync>> {
        let inner = self.inner.lock().unwrap();
        let child = inner.con.spawn_command_suspended(cmd)?;
        Ok(Box::new(child))
    }
}

#[cfg(test)]
mod conpty_input_contract_tests {
    use super::*;

    #[test]
    fn creation_and_resize_share_signed_dimension_bounds() {
        for (rows, cols) in [(0, 80), (25, 0), (u16::MAX, 80), (25, u16::MAX)] {
            assert!(validated_dimensions(rows, cols).is_err());
        }
        let maximum = validated_dimensions(32767, 32767).unwrap();
        assert_eq!(maximum.X, 32767);
        assert_eq!(maximum.Y, 32767);
    }

    #[test]
    fn invalid_dimensions_are_rejected_before_pipe_creation() {
        let system = ConPtySystem::default();
        for (rows, cols) in [(0, 80), (25, 0), (u16::MAX, 80), (25, u16::MAX)] {
            let size = PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            };
            assert!(
                system.openpty(size).is_err(),
                "invalid ConPTY dimensions {rows}x{cols} must be rejected"
            );
        }
    }
}
