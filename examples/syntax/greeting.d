module greeting;
import std.stdio;
// Compile-time constant and iteration.
void main() {
    immutable count = 3;
    foreach (i; 0 .. count) writeln("Hello ", i);
}
