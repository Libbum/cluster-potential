# cluster-potential

Removing giant IO overhead when building Wafer potentials from Gulp

## Requirements

Needs a local `gulp` executable in the cwd as well as the GULP_LIB (and probably GULP_DOC) environment variable(s) to be set. 
Two files `clusternn.xyz` and `cluster_wo_nn.xyz` must also reside in the cwd.
The first being nearest neighbours of a defect, and the second are second nearest neighbours only (i.e. a 2nn voronoi treatment with the nn's intersected out).

## Input adjustment

By default, running with no arguments will generate a file for node=1. 
to generate a file for node=13 (for example), run `cluster_potential 13`.

`a`, `num{x,y,z}`, `cpus` are all hardcoded. 
I don't think this will need to change ever, so there's no need to require user intervention on this right now. 
Change the variables and recompile if you need to.

