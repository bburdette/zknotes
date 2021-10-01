let windowkeysdebug = true;

let windowkeys = {};

function sendKeyCommand( kc ) {
  if (windowkeysdebug) {
    console.log("sendKeyCommand", kc);
  }

  if (kc.cmd == "SetWindowKeys") {
    windowkeys = {};
    for (let i = 0; i < kc.keys.length; i++) {
      k = kc.keys[i];
      console.log("k", k);
      if (!windowkeys[k.key]) {
        windowkeys[k.key] = {};
      }
      if (!windowkeys[k.key][k.ctrl]) {
        windowkeys[k.key][k.ctrl] = {};
      }
      if (!windowkeys[k.key][k.ctrl][k.alt]) {
        windowkeys[k.key][k.ctrl][k.alt] = {};
      }
      if (!windowkeys[k.key][k.ctrl][k.alt][k.shift]) {
        windowkeys[k.key][k.ctrl][k.alt][k.shift] = {};
      }
      windowkeys[k.key] [k.ctrl] [k.alt] [k.shift] = k.preventDefault
    }   
  }
}

function keycheck(e) {
  try {
    // console.log("e", e);
    // console.log("e.key", e.key, "e.ctrlKey", e.ctrlKey, "e.altKey", e.altKey, "e.shiftKey", e.shiftKey);
    let pd = windowkeys[e.key][e.ctrlKey][e.altKey][e.shiftKey];
    if (pd) {
      e.preventDefault();
    }
    console.log("pd:", pd);
    console.log("e", e);
    // app.ports.receiveKeyMsg.send(
  } catch (error)
  {
    console.log("not found: ", e.key);
  }


  // if (e.key === "Tab") {
  //   e.preventDefault();
  //   app.ports.tabKey.send("Tab");
  // }
}



