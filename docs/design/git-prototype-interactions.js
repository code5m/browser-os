// Shared, local-only interaction model for the two design prototypes.
(() => {
  const panel = document.querySelector('#rebasePanel');
  panel.style.cssText = 'position:relative;right:auto;bottom:auto;width:auto;max-width:none;max-height:45vh;overflow:auto;flex-shrink:0;box-shadow:none';
  const rows = [...panel.querySelectorAll('.rebase-row')];
  const actions = ['pick', 'reword', 'edit', 'squash', 'fixup', 'drop'];
  const todo = rows.reverse().map(row => ({ title: row.querySelector('span').textContent, action: 'pick' }));
  rows.forEach(row => row.remove());
  const list = document.createElement('div');
  const output = document.createElement('pre');
  output.setAttribute('aria-live', 'polite');
  output.style.whiteSpace = 'pre-wrap';
  panel.insertBefore(list, panel.querySelector('.action-menu,.button-row'));
  panel.append(output);
  panel.querySelector('p').textContent = '按从旧到新的顺序重放提交。用上下箭头排序，选择动作后生成计划。';
  function render() {
    list.replaceChildren();
    todo.forEach((item, index) => {
      const row = document.createElement('div');
      row.className = 'rebase-row';
      row.style.gridTemplateColumns = '90px 1fr auto auto';
      const select = document.createElement('select');
      select.setAttribute('aria-label', item.title + ' 的操作');
      actions.forEach(action => select.add(new Option(action, action)));
      select.value = item.action;
      select.onchange = () => { item.action = select.value; output.textContent = ''; };
      const title = document.createElement('span');
      title.textContent = item.title;
      row.append(select, title);
      [-1, 1].forEach(direction => {
        const button = document.createElement('button');
        button.textContent = direction < 0 ? '↑' : '↓';
        button.setAttribute('aria-label', (direction < 0 ? '上移 ' : '下移 ') + item.title);
        button.disabled = index + direction < 0 || index + direction >= todo.length;
        button.onclick = () => {
          [todo[index], todo[index + direction]] = [todo[index + direction], todo[index]];
          output.textContent = '';
          render();
        };
        row.append(button);
      });
      list.append(row);
    });
  }
  document.querySelector('#rebasePreview').onclick = () => {
    const remaining = todo.filter(item => item.action !== 'drop');
    if (!remaining.length) { output.textContent = '所有提交均被删除：计划为空，请保留至少一个提交。'; return; }
    if (['squash', 'fixup'].includes(remaining[0].action)) {
      output.textContent = '首个保留的提交不能 squash / fixup：它前面没有可合并的提交。';
      return;
    }
    output.textContent = '重放计划（示例，不修改仓库）\n\n' +
      todo.map(item => item.action + ' ' + item.title).join('\n') +
      '\n\n保留 ' + remaining.length + ' 条；删除 ' + (todo.length - remaining.length) + ' 条。';
  };
  render();
  // Collapse unchanged context rather than merely changing the status text.
  const toggle = document.querySelector('#context,#contextToggle');
  let changesOnly = false;
  function applyContext() {
    const body = document.querySelector('#editorBody');
    body.querySelectorAll('.split-diff p,.diff p,.code > span').forEach(line => {
      const changed = line.matches('.old,.new,.added,.removed');
      line.hidden = changesOnly && !changed;
    });
  }
  toggle.onclick = () => {
    changesOnly = !changesOnly;
    toggle.textContent = changesOnly ? '展开未变更行' : '折叠未变更行';
    toggle.setAttribute('aria-pressed', String(changesOnly));
    applyContext();
  };
  new MutationObserver(applyContext).observe(document.querySelector('#editorBody'), { childList: true });
  // Rebased defaults to the graph in the main editor; BrowserOS keeps its bottom history.
  const log = document.querySelector('#logPage');
  if (log) {
    const graph = log.querySelector('.bottom');
    const divider = log.querySelector('.hsplit');
    const content = log.querySelector('.content');
    const setPlacement = bottom => {
      if (bottom) {
        log.append(content, divider, graph);
        graph.style.flex = '0 0 var(--bottom-height,238px)';
        content.style.flex = '1';
      } else {
        log.append(graph, divider, content);
        graph.style.flex = '1';
        content.style.flex = '0 0 40%';
      }
    };
    document.querySelectorAll('input[name="logPlace"]').forEach((radio, index) => {
      radio.onchange = () => setPlacement(index === 1);
    });
    setPlacement(false);
  }
})();
