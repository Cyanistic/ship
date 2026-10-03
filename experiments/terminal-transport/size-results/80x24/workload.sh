stty -echo
cols=80; rows=24
printf '\033[2J\033[H'
fill_row() {
    line=$(printf '%03d:' "$r")
    j=4
    while [ "$j" -lt "$cols" ]; do
        line="$line$(((r+j)%10))"
        j=$((j+1))
    done
    printf '\033[%d;1H\033[38;2;%d;20;30;48;2;10;%d;50m%s\033[0m' "$y" "$((r%200+30))" "$((r%150+50))" "$line"
}
r=1
while [ "$r" -le "$rows" ]; do
    y=$r; fill_row
    r=$((r+1))
done
printf '\033[1;10H\033[1;38;2;220;30;40;48;2;10;60;100mcafé 界 é\033[0m'
printf '\033[%d;1HSIZE-READY\033[%d;1H' "$rows" "$rows"
read x
printf '\033[2;1H\033[3;4;38;2;10;20;30mEDIT\033[0m\033[%d;1H' "$rows"
read x
printf '\033[%d;1H\r\n' "$rows"
r=$((rows+1)); y=$rows; fill_row
printf '\033[%d;1HONE-SCROLL\033[%d;1H' "$rows" "$rows"
read x
exit 0
