using System;
// A tiny immutable record.
record Greeting(string Name) {
    public string Render() => $"Hello, {Name}!";
}
class Program {
    static void Main() => Console.WriteLine(new Greeting("reader").Render());
}
