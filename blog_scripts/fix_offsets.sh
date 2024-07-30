#!/usr/bin/bash
cwd=$(pwd)
target="dnd.png"
best=1000

offsets="
ogre,+3+7
slime,-2-5
golem,+1+7
insectoid,+0+9
lich,+3+3
lookseer,-3+1
king,+4+9
bear,+5+4
goblin,+1+15
minotaur,+3+11
demon,+3+17
goat,+1+7
skeleton,+1+3
kobold,+4+16
cultist,-4+7
squid,+1+4
imp,-1+6
chest,-4-2
"

# Recall the "tokyo" directory from earlier that houses all the sprites.
# Loop through each monster
for line in $offsets
do
    echo $line
    monster=$(echo $line | cut -f 1 -d ',')
    offset=$(echo $line | cut -f 2 -d ',')
    for frame in $(ls tokyo/$monster)
    do
        convert tokyo/$monster/$frame -background none -extent 32x32$offset tmp/fix_offset/$monster$frame
        convert tokyo/$monster/$frame -background none -extent 32x32$offset -alpha extract tmp/fix_offset/alpha$monster$frame
    done
done