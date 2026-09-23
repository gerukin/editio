<?php
// A typed function and string interpolation.
function greet(string $name): string {
    return "Hello, {$name}!";
}
echo greet('reader');
?>
