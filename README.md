# cluster-potential

Removing giant IO overhead when building Wafer potentials from Gulp

## Requirements

Needs a local `gulp` executable in the cwd as well as the GULP_LIB (and probably GULP_DOC) environment varable(s) to be set. 
A `cluster.xyz` file must also reside in the cwd.

## Input adjustment

By default, running with no arguments will generate a file for node=1. 
to generate a file for node=13 (for example), run `cluster_potential 13`.

`a`, `num{x,y,z}`, `cpus` are all hardcoded. 
I don't think this will need to change ever, so there's no need to require user intervention on this right now. 
Change the varables and recompile if you need to.

