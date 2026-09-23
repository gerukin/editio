// A case class and a collection transformation.
case class Reader(name: String)
object Greeting extends App {
  val readers = List(Reader("Ada"), Reader("Grace"))
  readers.foreach(r => println(s"Hello, ${r.name}!"))
}
