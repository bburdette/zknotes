let windowkeysdebug = false;

let windowkeys = {};

function sendKeyCommand( kc ) {
  if (windowkeysdebug) {
    console.log("sendKeyCommand", kc);
  }

  if (kc.cmd == "SetWindowKeys") {
    windowkeys = {};
    for (let i = 0; i < kc.keys.length; i++) {
      k = kc.keys[i];
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
    let pd = windowkeys[e.key][e.ctrlKey][e.altKey][e.shiftKey];
    if (pd) {
      e.preventDefault();
    }
    if (windowkeysdebug) {
      console.log("key found: ", e.key, " preventdefault: ", pd);
    }

    app.ports.receiveKeyMsg.send({ key : e.key
                                 , ctrl : e.ctrlKey
                                 , alt : e.altKey
                                 , shift : e.shiftKey
                                 , preventDefault : pd});
  } catch (error)
  {
    if (windowkeysdebug) {
      console.log("key not found: ", e.key);
    }
  }
}



