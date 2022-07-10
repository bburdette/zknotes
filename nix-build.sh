#  re the submodules!
#  you need a new version of nix to fetch the submodules. nix 2.8.0pre20220311_d532269 seems new enough.
#  in order to work on the repos, you need ssh github paths for the submodules.
#  but in order to build in flakes, you need https urls!
 
nix build  "git+file://$(pwd)?submodules=1"
