let localvalsdebug = false;

function storeVal( nv ) {
  localStorage.setItem(nv.name, nv.value);

  if (localvalsdebug) {
    console.log("js storeVal stored " + nv.name + ", " + nv.value);
  }
}

function getVal( gv ) {
  if (localvalsdebug) {
    console.log("js getVal getting " + gv.name + "," + localStorage.getItem(gv.name) + " for " + gv.for);
  }

  app.ports.localVal.send({ "for" : gv.for,
    "name": gv.name,
    "value": localStorage.getItem(gv.name)});
}

function clearStorage () {
  if (localvalsdebug) {
    console.log("clearstorage");
  }

  localStorage.clear();
}
