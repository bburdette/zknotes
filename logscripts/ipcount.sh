cat meh.txt | grep Some\(.*\) -o | grep \".*\" -o | sort | uniq -c | sort -h
