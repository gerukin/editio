# An XML builder template.
xml.instruct!
xml.readers do
  @readers.each do |reader|
    xml.reader reader.name, id: reader.id
  end
end
