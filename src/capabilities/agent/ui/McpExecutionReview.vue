<script setup lang="ts">
import { computed } from 'vue';
const props=defineProps<{review:any;approved:boolean;disabled?:boolean}>();
const emit=defineEmits<{(event:'update:approved',value:boolean):void}>();
const hasEnvironment=computed(()=>!!(props.review?.credential_refs?.length||props.review?.environment_names?.length));
</script>
<template><section v-if="review" class="mcp-execution-review" aria-label="MCP 实际执行范围">
  <strong>本次启动与调用范围</strong>
  <dl><dt>程序</dt><dd>{{review.command}}</dd><dt>启动参数</dt><dd><pre>{{JSON.stringify(review.args,null,2)}}</pre></dd>
  <dt>配置来源</dt><dd>{{review.source}}</dd><dt>工作区</dt><dd>{{review.workspace}}</dd>
  <dt>运行目录</dt><dd>{{review.cwd}}</dd><dt>超时</dt><dd>{{review.timeout_seconds}} 秒</dd></dl>
  <p>{{review.isolation==='appcontainer_no_network'?'在无网络 AppContainer 中运行，输入文件复制到私有目录。':'以当前用户运行本机进程，请确认程序及其访问范围。'}}</p>
  <p v-if="review.tool">工具：{{review.tool}}</p><pre v-if="review.tool">{{JSON.stringify(review.arguments,null,2)}}</pre>
  <p v-if="review.environment_names?.length">用户配置环境变量：{{review.environment_names.join('、')}}（值不展示）</p>
  <ul v-if="review.credentials?.length"><li v-for="item in review.credentials" :key="item.environment">
    {{item.environment}} 将取得模型配置「{{item.name}}」的凭据（{{item.endpoint}}）。
  </li></ul>
  <label v-if="hasEnvironment"><input type="checkbox" :checked="approved" :disabled="disabled"
    @change="emit('update:approved',($event.target as HTMLInputElement).checked)">我允许本次进程取得以上凭据和配置环境变量</label>
  <p v-else>此配置不请求模型凭据或自定义环境变量。</p>
  <p>配置、参数或凭据版本变化后，本次批准将失效。</p>
</section></template>
<style scoped>
.mcp-execution-review{border:1px solid #64728b;border-radius:5px;padding:8px;margin:8px 0;font-size:12px;overflow-wrap:anywhere}
dl{margin:6px 0}dt{font-weight:600;margin-top:5px}dd{margin:2px 0}pre{white-space:pre-wrap;max-height:160px;overflow:auto;font-size:11px}ul{padding-left:18px}
</style>
