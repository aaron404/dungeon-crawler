#!/usr/bin/bash
cwd=$(pwd)
target="dnd.png"
best=1000

# Recall the "tokyo" directory from earlier that houses all the sprites.
# Loop through each monster
for monster in $(ls tokyo)
do
    echo "checking $monster"

    # Loop an offset value for the sprite. I've chosen bounds that roughly align with the 
    # game board to avoid searching in areas where the sprite couldn't possibly be.
    for x in $(seq 25 300)
    do
        for y in $(seq 140 420)
        do
            # Use 'composite' to overlay the sprite on the main board screenshot
            composite $cwd/tokyo/$monster/0.png -geometry +$x+$y $target tmp/over.png
            # Compute a match score using 'mean squared error' metric. It outputs to stderr
            # for some reason hence the extra pipe wrangling. 'cut' pulls out the relevant
            # data from the output for doing numerical comparison
            score=$(compare dnd.png tmp/over.png -metric MSE tmp/dif.png 2>&1 > /dev/null | cut -f 1 -d '.')
            if [[ $score -lt $best ]]
            then
                best=$score
                echo "new best: $x $y $score"
                cp tmp/dif.png tmp/best.png
            fi
        done
    done
done