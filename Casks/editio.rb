cask "editio" do
  version "0.1.1"
  arch arm: "aarch64", intel: "x86_64"
  sha256 arm: "a0894232fb571a1f5ed0c716e55769c6cc1e37fc8c28bdfedb5625f5b9f2f7a2", intel: "53333696d0a132613505bb8d5af4b53533f850fb3225ccfda690469a623f3205"
  url "https://github.com/gerukin/editio/releases/download/v#{version}/editio-#{version}-#{arch}-apple-darwin.tar.gz"
  name "Editio"
  desc "Fast, minimal terminal text viewer and editor"
  homepage "https://github.com/gerukin/editio"
  depends_on macos: :big_sur
  binary "editio"
end
