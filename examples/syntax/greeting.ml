(* Pattern matching and lists. *)
let greet name = "Hello, " ^ name
let describe = function
  | [] -> "Nobody yet"
  | first :: _ -> greet first
let () = print_endline (describe ["Ada"; "Grace"])
