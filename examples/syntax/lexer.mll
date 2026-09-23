{ type token = Number of int | Plus | End }
rule token = parse
  | [' ' '\t'] { token lexbuf }
  | ['0'-'9']+ as n { Number (int_of_string n) }
  | '+' { Plus }
  | eof { End }
