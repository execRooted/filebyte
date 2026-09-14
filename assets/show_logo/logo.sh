#!/bin/bash



clear

delay=0.010
file="logo.txt"

if [[ ! -f "$file" ]]; then
    echo "Error: $file not found in $(pwd)" >&2
    exit 1
fi

terminal_width=$(tput cols 2>/dev/null)

if ! [[ "$terminal_width" =~ ^[0-9]+$ ]] || (( terminal_width == 0 )); then
    terminal_width=80
fi

logo_width=$(awk '
{
    if (length($0) > max)
        max = length($0)
}
END {
    print max + 0
}' "$file")

padding=$(( (terminal_width - logo_width) / 2 ))

if (( padding < 0 )); then
    padding=0
fi

while IFS= read -r line || [[ -n "$line" ]]; do
    printf "%*s" "$padding" ""
    printf "\033[34m"
    while IFS= read -r -n1 char || [[ -n "$char" ]]; do
        printf "%s" "$char"
        if [[ "$char" != " " ]]; then
            sleep "$delay"
        fi
    done <<< "$line"
    printf "\033[0m\n"
done < "$file"
