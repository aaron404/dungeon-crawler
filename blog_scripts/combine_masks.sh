#!/usr/bin/bash

set -e

sprite_width=38
sprite_height=48
margin=3

frame=0

dst=tmp/combine_masks
start=$dst/start.png
mask=$dst/mask.png
ghost=$dst/ghost.png

width=$((2 * $sprite_width + 4 * $margin))
height=$(($sprite_height + 2 * $margin))

offsets="
demon,+3+17
minotaur,+3+11
bear,+5+4
ogre,+3+7
golem,+1+7
lich,+3+3
insectoid,+0+9
king,+4+9
goblin,+1+15
lookseer,-3+1
skeleton,+1+3
goat,+1+7
cultist,-4+7
kobold,+4+16
squid,+1+4
imp,-1+6
slime,-2-5
chest,-4-2
"

rm $dst/*

echo "canvas size: $width x $height"
convert -size "$width x $height" canvas:transparent $start
rect_x1=$((3 * $margin + $sprite_width + $sprite_width / 2 - 16))
rect_y1=$(($margin + $sprite_height / 2 - 16))
rect_x2=$(($rect_x1 + 31))
rect_y2=$(($rect_y1 + 31))
convert $start -fill white -draw "rectangle $rect_x1 $rect_y1 $rect_x2 $rect_y2" $start

for line in $offsets
do
    monster=$(echo $line | cut -d ',' -f 1)
    offset_x=$(echo $line | cut -d ',' -f 2)
    offset_y=$(echo $line | cut -d ',' -f 3)

    # src=tokyo/$monster/0.png
    src=tmp/fix_offset/${monster}0.png
    info=$(identify $src | cut -d ' ' -f 3)
    monster_w=$(echo $info | cut -d 'x' -f 1)
    monster_h=$(echo $info | cut -d 'x' -f 2)

    # position of sprite when it is centered in the box
    sprite_x=$(($margin + $sprite_width / 2 - $monster_w / 2))
    sprite_y=$(($margin + $sprite_height / 2 - $monster_h / 2))

    echo "  $monster $monster_w x $monster_h, $sprite_x"

    convert $src -alpha extract $mask
    convert $mask -alpha set -channel a -evaluate set 50% $ghost
    # convert $src -threshold 99% -alpha extract -alpha on $mask
    convert $src -channel alpha -threshold 99% +channel -alpha extract -alpha on $mask

    # dist_x=$(($monster_w + $sprite_width / 2 + 2 * $margin))
    dist_x=$(($monster_w + $sprite_width + 2 * $margin))
    dist_y=$(($monster_h + $monster_h / 2 + $sprite_height / 2 + $margin))
    dist=$(($dist_x > $dist_y ? $dist_x : $dist_y))

    # for y in $(seq -$monster_h $height)
    for d in $(seq 0 $(($dist + $sprite_y)))
    do
        printf -v i "%05d" $frame

        y=$(($d - $monster_h))

        if [[ $y -ge $sprite_y ]]
        then
            d_tmp=$(($y - $sprite_y))
            d_tmp=$(($d_tmp < $dist_x ? $d_tmp : $dist_x))
            x=$(($d_tmp + $sprite_x))
            magick composite $ghost $start -geometry +$x+$sprite_y -compose over $dst/$i.png
        else
            cp $start $dst/$i.png
        fi

        composite $src $dst/$i.png -geometry +$sprite_x+$y -compose over $dst/$i.png

        frame=$((frame + 1))

        if [[ $d -eq $(($dist + $sprite_y)) ]]
        then
            # bake mask in
            echo "baking"
            composite $mask $start -geometry +$x+$sprite_y -compose dst-in $start
            printf -v i "%05d" $frame
            cp $start $dst/$i.png
            frame=$((frame + 1))
        fi
    done
done

for n in $(seq 0 100)
do
    printf -v i "%05d" $frame
    cp $(ls $dst/0*.png | tail -n1) $dst/$i.png
    frame=$((frame + 1))
done

for n in $(seq 0 2 9)
do
    rm $dst/0*$n.png
done

convert $dst/0*.png -filter point -resize 400% -quality 100 $dst/_combine_masks.webm