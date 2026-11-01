// Shared Monaco model identity, language selection and versioned document helpers.
// The business owner alone controls authorized file saves and approval.
export function editorLanguage(path) {
  const extension = String(path || '').split('/').pop()?.split('.').pop()?.toLowerCase();
  const langs = {ts:'typescript',tsx:'typescript',js:'javascript',jsx:'javascript',
    rs:'rust',py:'python',java:'java',sql:'sql',md:'markdown',html:'html',
    css:'css',scss:'scss',json:'json',yaml:'yaml',yml:'yaml',sh:'shell',
    toml:'plaintext',txt:'plaintext',vue:'html'};
  return langs[extension] || 'plaintext';
}
export function createDocumentModel(monaco,{path,content,language,namespace,scheme='browseros-document'}){
  if (!monaco?.editor?.createModel || !monaco?.Uri?.from) throw Error('EDITOR_MONACO_REQUIRED');
  if (!path || !namespace) throw Error('EDITOR_DOCUMENT_IDENTITY_REQUIRED');
  const uri=monaco.Uri.from({scheme,authority:namespace,path:path.startsWith('/')?path:'/'+path});
  if (monaco.editor.getModel(uri)) throw Error('EDITOR_DOCUMENT_ALREADY_OPEN');
  return monaco.editor.createModel(content??'',language||editorLanguage(path),uri);
}
export function applyVersionedDocumentEdit(model,expectedVersion,content){
  if(model.isDisposed())throw Error('EDITOR_DOCUMENT_CLOSED');
  if(model.getVersionId()!==expectedVersion)throw Error('EDITOR_DOCUMENT_VERSION_CONFLICT');
  if(model.getValue()===content)return model.getVersionId();
  model.setValue(content);
  return model.getVersionId();
}
