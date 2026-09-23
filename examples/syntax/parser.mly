%token <int> NUMBER
%token PLUS EOF
%left PLUS
%start main
%type <int> main
%%
main: expression EOF { $1 };
expression: NUMBER { $1 } | expression PLUS expression { $1 + $3 };
