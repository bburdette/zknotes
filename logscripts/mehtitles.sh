cat meh.txt | grep $1 | grep " - .* - " -o | sed s/\ -\ //g | sort | uniq -c
