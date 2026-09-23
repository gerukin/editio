# Display-only syntax fixture; never imported or executed.
def greet(name: str) -> str:
    return f"Hello, {name}!"

names = ["Ada", "Grace"]
for name in names:
    print(greet(name))
