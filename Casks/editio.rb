cask "editio" do
  version "0.1.2"
  arch arm: "aarch64", intel: "x86_64"
  sha256 arm: "73bd94081a406cbc7b3cf6ff4e7514ff61c2f4f55ed993d11acc320a86137ed0", intel: "fe339a10795d4bcdd55c212cc8a9bd60fdf73732a7f66436fafb359d37fefa7e"
  url "https://github.com/gerukin/editio/releases/download/v#{version}/editio-#{version}-#{arch}-apple-darwin.tar.gz"
  name "Editio"
  desc "Fast, minimal terminal text viewer and editor"
  homepage "https://github.com/gerukin/editio"
  depends_on macos: :big_sur
  binary "editio"
end
