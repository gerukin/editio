// Functions, interpolation, and an object literal.
const greet = (name) => `Hello, ${name}!`;
const settings = { tabSize: 4, enabled: true };
for (const name of ["Ada", "Grace"]) {
  console.log(greet(name));
}
