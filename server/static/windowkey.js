// this will contain the keys we're monitoring.
let windowkeys = {};

// wire up your sendKeyCommand port in elm to this function:
//    app.ports.sendKeyCommand.subscribe(sendKeyCommand);
function sendKeyCommand( kc ) {
  // console.log("sendKeyCommand", kc);

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

// add this line after your elm app init.
// window.addEventListener( "keydown", keycheck, false );
function keycheck(e) {
  try {
    let pd = windowkeys[e.key][e.ctrlKey][e.altKey][e.shiftKey];
    if (pd) {
      e.preventDefault();
    }
    // console.log("key found: ", e.key, " preventdefault: ", pd);

    app.ports.receiveKeyMsg.send({ key : e.key
                                 , ctrl : e.ctrlKey
                                 , alt : e.altKey
                                 , shift : e.shiftKey
                                 , preventDefault : pd});
  } catch (error)
  {
   // console.log("key not found: ", e.key);
  }
}

