i=0
while [ "$i" -lt 180 ]; do
    printf 'progress line %03d: abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789\n' "$i"
    i=$((i + 1))
done
printf 'ASSERT budget_total: expected 17, observed 19\n'
exit 1
