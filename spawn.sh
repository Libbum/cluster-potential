#!/bin/bash

for a in {1..30}; do
    mkdir node$a;
    cd node$a;
    for b in {1..48}; do
        mkdir chunk$b;
        cd chunk$b;
        cp ~/bin/gulp .;
        cp ../../clusternn.xyz .;
        cp ../../cluster2nn_wo_nn.xyz .;
        cp ../../cluster-potential .;
        cp ../../gulp.job pot$a.job; 
        sed -i 's/node=1/node='$a'/' pot$a.job; 
        sed -i 's/chunk=1/chunk='$b'/' pot$a.job; 
        sed -i 's/runname/l1_'$a'_'$b'/' pot$a.job; 
        qsub pot$a.job; 
        cd ..;
    done
    cd ..; 
done

