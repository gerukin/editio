# Procedures and substitution.
proc greet {name} {
    return "Hello, $name!"
}
foreach name {Ada Grace} {
    puts [greet $name]
}
