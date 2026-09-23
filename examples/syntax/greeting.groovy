// Closures and string interpolation.
def greet = { name -> "Hello, ${name}!" }
def names = ['Ada', 'Grace']
names.each { println greet(it) }
