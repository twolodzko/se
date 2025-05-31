#!/usr/bin/env bats

is_gsed() {
   sed --version >/dev/null 2>&1
}

setup() {
   if ! is_gsed && command -v gsed >/dev/null 2>&1 ; then
      sed() {
         gsed "$@"
      }
   fi
}

teardown() {
    rm -f /tmp/script.sed
    rm -f /tmp/{a,b,c}.txt
}

@test "Fails with no arguments" {
	! ./se
}

@test "Using q command results in proper error code" {
	run ./se 'q 13' README.md
	[ "$status" -eq 13 ]
}

@test "Count works" {
	[ $(./se -c '/the/' README.md) -eq $(grep -c 'the' README.md) ]
}

@test "Print all" {
	run diff <(./se 'p' README.md) <(cat README.md)
   [ "$status" -eq 0 ]
}

@test "Print vs Println" {
	run diff <(./se 'p' README.md) <(./se 'P"\n"' README.md)
   [ "$status" -eq 0 ]
}

@test "Print all with -a and no command" {
	run diff <(./se -a '' README.md) <(cat README.md)
   [ "$status" -eq 0 ]
}

@test "Group of commands" {
	run diff <(./se '1ppp' README.md) <(sed -n '1 {p;p;p;}' README.md)
   [ "$status" -eq 0 ]
}

@test "Delete lines" {
	run diff <(./se -a '/sed/ d' README.md) <(sed '/sed/ d' README.md)
   [ "$status" -eq 0 ]
}

@test "Use negation" {
   run diff <(./se '(1-3)! p' README.md) <(tail -n +4 README.md)
   [ "$status" -eq 0 ]
}

@test "Use negation with set" {
   run diff <(./se '(1,2,3)! p' README.md) <(tail -n +4 README.md)
   [ "$status" -eq 0 ]
}

@test "Count is consistent with number of printed lines" {
	[ $(./se -c '/the/' README.md) -eq $(./se '/the/ p' README.md | wc -l | sed 's/ *//') ]
}

@test "Consistent with sed line counting" {
   run diff <(sed '=' README.md) <(./se '="\n"p' README.md)
   [ "$status" -eq 0 ]
}

@test "Special characters in template" {
   run diff <(./se '="\n"p' README.md) <(./se '="\n"p' README.md)
   [ "$status" -eq 0 ]
}

@test "Simple substitute is like in sed" {
   run diff <(sed 's/a/#/g' README.md) <(./se -a 's/a/#/' README.md)
   [ "$status" -eq 0 ]
}

@test "Simple substitute with one replacement is like in sed" {
   run diff <(sed 's/a/#/' README.md) <(./se -a 's/a/#/1' README.md)
   [ "$status" -eq 0 ]
}

@test "Substitute and print vs sed" {
   run diff <(sed -n 's/a/#/gp' README.md) <(./se '/a/ s/a/#/p' README.md)
   [ "$status" -eq 0 ]
}

@test "Print selected lines like in sed" {
   run diff <(sed -n '3,/address/ p' README.md) <(./se '3-/address/ p' README.md)
   [ "$status" -eq 0 ]
}

@test "Print head using line matching" {
   run diff <(head -n 5 README.md) <(./se '1-5p.q' README.md)
   [ "$status" -eq 0 ]
}

@test "Print head using repeated read" {
   run diff <(head -n 5 README.md) <(./se 'r4 p q' README.md)
   [ "$status" -eq 0 ]
}

@test "Print tail" {
   run diff <(tail -n 5 README.md) \
            <(./se '1 r4x . x s/[^\n]*\n(.*)/$1/1 jx . $ xp' README.md)
   [ "$status" -eq 0 ]
}

@test "Be like cut" {
   run diff <(cut -c '2-7' src/main.rs) <(./se 'k2-7p' src/main.rs)
   [ "$status" -eq 0 ]
}

@test "Replace all like in sed" {
   run diff <(sed -nE 's/in (`sed`)/__&__/p' README.md) <(./se '/in `sed`/ s/in (`sed`)/__$0__/p' README.md)
   [ "$status" -eq 0 ]
}

@test "Replace captured group like in sed" {
   run diff <(sed -nE 's/in (`sed`)/__\1__/p' README.md) <(./se '/in `sed`/ s/in (`sed`)/__$1__/p' README.md)
   [ "$status" -eq 0 ]
}

@test "Count lines like sed" {
   run diff <(sed -n '$=' README.md) <(./se '$="\n"' README.md)
   [ "$status" -eq 0 ]
}

@test "Stop early" {
   run diff <(se '7=q' README.md) <(printf "7")
   [ "$status" -eq 0 ]
}

@test "Eval works" {
   run diff <(pwd) <(./se 'ep' <(echo 'printf $(pwd)'))
   [ "$status" -eq 0 ]
}

@test "Count matching lines like grep" {
   run diff <(grep -c 'sed' README.md) <(./se -c '/sed/' README.md)
   [ "$status" -eq 0 ]
}

@test "Flags work" {
   # no flag
   run diff <(sed -n '/Address/p' README.md) <(./se '/Address/p' README.md)
   [ "$status" -eq 0 ]

   # with flag
   run diff <(sed -n '/Address/Ip' README.md) <(./se '/(?i)Address/p' README.md)
   [ "$status" -eq 0 ]
}

@test "Whole line syntax" {
   run diff <(./se '/^## Commands$/ p' README.md) <(./se '^## Commands$ p' README.md)
   [ "$status" -eq 0 ]
}

@test "Empty regex in address" {
   run diff <(./se '// p' README.md) <(cat README.md)
   [ "$status" -eq 0 ]
}

create_commented_script() {
cat <<EOF > /tmp/script.sed
   # start of script
   /(?x)
            # using verbose mode
      se    # so those comments would be ignored
      d\`
      \     # you need to escape whitespace
            # you can insert //\/\\\##\#/q42/ in comments without fear
   /        # but comments are also possible outside
   p
   # end of script
EOF
}

@test "Use script file with comments" {
   run create_commented_script
   run diff <(./se -f /tmp/script.sed README.md) <(./se '/sed\` /p' README.md)
   [ "$status" -eq 0 ]
}

@test "Use hold buffer to delay printing lines" {
   run diff <(sed -n '{x;p;}' README.md) <(./se 'xp' README.md)
   [ "$status" -eq 0 ]
}

@test "Use hold and pattern buffers" {
   run diff <(sed -n '7h ; 8{x;G;h} ; 9{x;G;p}' README.md) \
            <(./se '7h; 8xjh; 9xjp' README.md)
   [ "$status" -eq 0 ]
}

@test "Condition on substitute like sed" {
   run diff <(sed -nE 's/(sed)/__\1__/gp' README.md) \
            <(./se '? s/(sed)/__$1__/p' README.md)
   [ "$status" -eq 0 ]
}

bash_line_marker() {
   while IFS="" read -r line || [ -n "$line" ]
   do
      if [[ "$line" =~ "sed" ]]; then
         printf '>>> %s\n' "$line"
      elif [[ "$line" =~ "the" ]]; then
         printf '*** %s\n' "$line"
      else
         printf '    %s\n' "$line"
      fi
   done <README.md
}

@test "The stop behavior works as intended" {
   run diff <(bash_line_marker) <(./se '/sed/ ">>> " p . /the/ "*** " p . "    " p' README.md)
   [ "$status" -eq 0 ]
}

only_for_gsed() {
   if ! is_gsed ; then
      skip "This works only on GNU Sed"
   fi
}

@test "Clear buffer like gsed" {
   only_for_gsed
   run diff <(./se -a '/sed/ z' README.md) <(sed '/sed/ z' README.md)
   [ "$status" -eq 0 ]
}

@test "Append text like gsed" {
   only_for_gsed
   run diff <(sed '/sed/a >>>' README.md) <(./se '/sed/ p ">>>\n" . p' README.md)
   [ "$status" -eq 0 ]
}

@test "Insert text like gsed" {
   only_for_gsed
   run diff <(sed '/sed/i >>>' README.md) <(./se '/sed/ ">>>\n" p . p' README.md)
   [ "$status" -eq 0 ]
}

@test "Multiple input files" {
   echo 1 > /tmp/a.txt
   echo 2 > /tmp/b.txt
   echo 3 > /tmp/c.txt

   run diff <(./se 'p' /tmp/a.txt /tmp/b.txt /tmp/c.txt) <(printf "1\n2\n3\n")
   [ "$status" -eq 0 ]

   echo 'p' > /tmp/script.sed
   run diff <(./se -f /tmp/script.sed /tmp/a.txt /tmp/b.txt /tmp/c.txt) <(printf "1\n2\n3\n")
   [ "$status" -eq 0 ]
}

@test "Run the examples in README.md" {
   run sed -nE 's/^.*`(se .+ README.md)`.*/.\/\1/e' README.md
   [ "$status" -eq 0 ]
}
