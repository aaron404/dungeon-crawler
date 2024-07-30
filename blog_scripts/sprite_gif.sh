#!/usr/bin/bash

cwd=$(pwd)
# root_dir=tokyo/monsters
root_dir=Content/Packed/textures/tokyo/monsters
work_dir=tokyo/original

# clear working dir and copy sprites there
mkdir -p $work_dir
rm -r work_dir/* 2>/dev/null
for monster in $(ls $cwd/$root_dir)
do
    mkdir -p tokyo/$monster
    cp $cwd/$root_dir/$monster/idle/* $work_dir/$monster
done

mkdir -p tmp
rm -r tmp/* 2>/dev/null

for monster in $(ls $work_dir)
do
    img_dir=$cwd/$work_dir/$monster
    cd $img_dir

    # remove duplicates
    for hash in $(sha256sum * | sort | cut -f 1 -d ' ' | uniq)
    do
        for dupe in $(sha256sum * | grep $hash | tail -n +2 | cut -f 3 -d ' ')
        do
          rm $dupe
        done
    done

    count=$(ls *.png | wc -l)
    w=$(file *.png | cut -d ' ' -f 5 | sort | tail -1 | rg -o "\d+")
    h=$(file *.png | cut -d ' ' -f 7 | sort | tail -1 | rg -o "\d+")
    echo "$monster: $count frames, $w x $h"

    # rename images
    i=0
    for f in $(ls)
    do
        mv $f $i.png
        i=$((i + 1))
    done
    
    # resize images according to max width/height
    for n in $(seq 0 $((count - 1)))
    do
        # echo "  $n"
        convert $n.png -gravity southeast -background none -extent "$w x $h" $n.png
    done

    for n in $(seq 0 7)
    do
        # if [ ! -f "0$n.png" ]; then
        #     cp 00.png "$cwd/tmp/0$n$monster.png"
        # else
        #     cp "0$n.png" "$cwd/tmp/"
        # fi
        dst=$cwd/tmp/$n$monster.png
        if [ -f "$n.png" ]; then
            src="$n.png"
        else
            src=0.png
        fi
        convert $src -background none -bordercolor black -compose copy -border 1 $dst
    done
done

for i in $(seq 0 7)
do
    montage -background none -mode concatenate -gravity southeast -tile 6x $cwd/tmp/$i*.png  $cwd/tmp/_$i.png
done

convert -background none $cwd/tmp/_*.png -quality 100 $cwd/tmp/sprites_all.webm

# cd $cwd
# exit

# sleep 2

# for i in {1..8}
# do
#     montage -background none -mode concatenate -gravity southeast -tile 6x $(fdfind $i.png $cwd/$root_dir/*/idle) $cwd/gifs/$i.png
#     convert $cwd/gifs/$i.png -filter point -resize 200% $cwd/gifs/$i.png
# done