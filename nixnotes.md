
build the package:

```
$ nix build
```

if you get an elm error, you may need to update the nix deps.

```
$ cd elm
/elm$ elm2nix convert > elm-srcs.nix
/elm$ elm2nix snapshot
```

Once nix build completes, you can copy the package to a remote server:

```
nix-copy-closure --to myuser@myserver.com $(readlink result)
```

I run my server in a user account, so I'll ssh in to that and install the new package:

```
nix-env -i /nix/store/30lqrvj2d3rcwrk5r966mwdnxc6dxhc1-zknotes
```

Then restart the service from root and it will switch over to the new version.

```
systemctl restart zknotes.service
```
