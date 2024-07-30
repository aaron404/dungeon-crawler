#!/usr/bin/bash

set -e 
board_img=monster_refs/demon.png
sprite=demon0.png

range=14

rm tmp/align*

i=0
for y in $(seq $((257 - range)) 2 $((257 + range)))
do
    for x in $(seq $((211 - range)) 2 $((211 + range)))
    do
        printf -v j "%03d" $i
        composite demon0.png monster_refs/demon.png -geometry +$x+$y tmp/align$j.png
        convert tmp/align$j.png -crop 67x80+196+241 -filter point -resize 300% tmp/align$j.png
        convert tmp/align$j.png -draw "text 0,10 'abc'" tmp/align$j.png
        i=$((i + 1))

        if [[ $x -eq 211 ]]
        then
            if [[ $y -eq 257 ]]
            then
                for k in $(seq 1 30)
                do
                    printf -v j "%03d" $i
                    composite demon1.png monster_refs/demon.png -geometry +209+255 tmp/align$j.png
                    convert tmp/align$j.png -crop 67x80+196+241 -filter point -resize 300% tmp/align$j.png
                    i=$((i + 1))
                done
            fi
        fi
    done
done

convert tmp/align*.png -quality 100 tmp/align.webm