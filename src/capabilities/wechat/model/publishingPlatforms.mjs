// Official, allowlisted console entrypoints. No tokens, cookies, passwords or
// provider-specific DOM assumptions are persisted in this platform catalog.
export const PUBLISHING_PLATFORMS = Object.freeze([
  {id:"wechat",name:"微信公众号",icon:"📣",category:"图文",url:"https://mp.weixin.qq.com/",officialHost:"mp.weixin.qq.com",integration:"wechat-draft"},
  {id:"baijiahao",name:"百度百家号",icon:"🔎",category:"图文",url:"https://baijiahao.baidu.com/",officialHost:"baijiahao.baidu.com",integration:"console"},
  {id:"toutiao",name:"头条号",icon:"📰",category:"图文",url:"https://mp.toutiao.com/",officialHost:"mp.toutiao.com",integration:"console"},
  {id:"xiaohongshu",name:"小红书",icon:"📕",category:"图文/视频",url:"https://creator.xiaohongshu.com/",officialHost:"creator.xiaohongshu.com",integration:"console"},
  {id:"bilibili",name:"哔哩哔哩",icon:"📺",category:"视频",url:"https://member.bilibili.com/platform/home",officialHost:"member.bilibili.com",integration:"console"},
  {id:"douyin",name:"抖音",icon:"🎬",category:"短视频",url:"https://creator.douyin.com/",officialHost:"creator.douyin.com",integration:"console"},
  {id:"kuaishou",name:"快手",icon:"🎥",category:"短视频",url:"https://cp.kuaishou.com/",officialHost:"cp.kuaishou.com",integration:"console"},
  {id:"sohu",name:"搜狐号",icon:"🗞️",category:"图文",url:"https://mp.sohu.com/",officialHost:"mp.sohu.com",integration:"console"},
  {id:"weibo",name:"微博",icon:"✺",category:"社交",url:"https://weibo.com/",officialHost:"weibo.com",integration:"console"},
  {id:"zhihu",name:"知乎",icon:"✒️",category:"问答/专栏",url:"https://www.zhihu.com/",officialHost:"www.zhihu.com",integration:"console"}
].map(Object.freeze));
export function platformById(id) { return PUBLISHING_PLATFORMS.find(p=>p.id===id) ?? null; }
export function officialPublisherUrl(id) { return platformById(id)?.url ?? null; }
export function matchesOfficialHost(id,input) {
  const platform=platformById(id);
  if(!platform || typeof input!=="string" || input.length>2048) return false;
  try {
    const u=new URL(input);
    return u.protocol==="https:" && u.hostname===platform.officialHost && !u.username && !u.password;
  } catch { return false; }
}
