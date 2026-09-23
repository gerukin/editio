-- A small AppleScript handler.
on greet(person)
    set message to "Hello, " & person
    return message
end greet
greet("reader")
