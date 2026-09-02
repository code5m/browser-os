<script setup lang="ts">
// M1-9 关闭协议弹窗：关闭页签时必须显式选择「保存 / 删除 / 取消」。
// 「不可静默丢」由本弹窗保证；异常退出时未决定的草稿不落盘。
import { useSessionStore } from "../../stores/useSessionStore";

const session = useSessionStore();
</script>

<template>
  <div v-if="session.closeDialogOpen" class="sc-mask">
    <div class="sc-dialog">
      <div class="sc-title">关闭页签</div>
      <div class="sc-desc">
        即将关闭「{{ session.pendingCloseTitle }}」。是否保存本次浏览的会话记录？
      </div>
      <div class="sc-note">
        保存 = 会话（已脱敏 URL + 资源瀑布）存入本地历史会话；删除 = 会话丢弃，不落盘。
      </div>
      <div class="sc-actions">
        <button class="sc-btn primary" @click="session.resolveClose('save')">保存并关闭</button>
        <button class="sc-btn danger" @click="session.resolveClose('discard')">删除并关闭</button>
        <button class="sc-btn" @click="session.resolveClose('cancel')">取消</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.sc-mask {
  position: fixed;
  inset: 0;
  z-index: 1200;
  background: rgba(0, 0, 0, 0.45);
  display: flex;
  align-items: center;
  justify-content: center;
}
.sc-dialog {
  width: 380px;
  background: var(--panel-bg, #252526);
  border: 1px solid var(--border, #3c3c3c);
  border-radius: 8px;
  padding: 16px;
  color: var(--fg, #d4d4d4);
  box-shadow: 0 8px 30px rgba(0, 0, 0, 0.5);
}
.sc-title {
  font-size: 15px;
  font-weight: 600;
  margin-bottom: 8px;
}
.sc-desc {
  font-size: 13px;
  line-height: 1.6;
  word-break: break-all;
}
.sc-note {
  font-size: 12px;
  color: var(--fg-dim, #999);
  margin-top: 8px;
  line-height: 1.5;
}
.sc-actions {
  display: flex;
  gap: 8px;
  margin-top: 16px;
  justify-content: flex-end;
}
.sc-btn {
  border: 1px solid var(--border, #3c3c3c);
  background: none;
  color: inherit;
  border-radius: 5px;
  padding: 6px 12px;
  cursor: pointer;
  font-size: 13px;
}
.sc-btn.primary {
  background: var(--accent, #2b6cb0);
  border-color: var(--accent, #2b6cb0);
  color: #fff;
}
.sc-btn.danger {
  border-color: #a03d3d;
  color: #e08a8a;
}
</style>
