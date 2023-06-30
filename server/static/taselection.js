
function getTASelection(request) {
  var range = { text: "", offset: null };
  var activeEl = document.getElementById(request.id);
  var activeElTagName = activeEl ? activeEl.tagName.toLowerCase() : null;
  if (
    (activeElTagName == "textarea") || (activeElTagName == "input" &&
    /^(?:text|search|password|tel|url)$/i.test(activeEl.type)) &&
    (typeof activeEl.selectionStart == "number")
  ) {
      range.text = activeEl.value.slice(activeEl.selectionStart, activeEl.selectionEnd);
      range.offset = activeEl.selectionStart;
      range.what = request.what;
      app.ports.receiveTASelection.send(range);
  }
  else {
      app.ports.receiveTASelection.send(null);
  }
}

function setTASelection(request) {
  var activeEl = document.getElementById(request.id);
  var activeElTagName = activeEl ? activeEl.tagName.toLowerCase() : null;
  if (
    (activeElTagName == "textarea") || (activeElTagName == "input" &&
    /^(?:text|search|password|tel|url)$/i.test(activeEl.type)) &&
    (typeof activeEl.selectionStart == "number")
  ) {
      activeEl.setSelectionRange(request.offset, request.offset + request.length);
      activeEl.focus();
  }
}

