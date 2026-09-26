cask "editio" do
  version "0.1.0"
  arch arm: "aarch64", intel: "x86_64"
  sha256 arm: "30072e2431fee0ee70e3bffe9b9ed585f9c361c17aa42a9462f0b8fda4ca9865",
         intel: "44d90f444f1fe63865ed193cf3bdaa37b9dedaf472684b18e05a9c5ffa6cf5ea"
  url "https://github.com/gerukin/editio/releases/download/v#{version}/editio-#{version}-#{arch}-apple-darwin.tar.gz"
  name "Editio"
  desc "Fast, minimal terminal text viewer and editor"
  homepage "https://github.com/gerukin/editio"
  depends_on macos: :big_sur
  binary "editio"
end
