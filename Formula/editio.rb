class Editio < Formula
  desc "Fast, minimal terminal text viewer and editor"
  homepage "https://github.com/gerukin/editio"
  version "0.1.0"
  license "MIT"

  on_linux do
    on_intel do
      url "https://github.com/gerukin/editio/releases/download/v0.1.0/editio-0.1.0-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "44a8b5d638803304925f51999c43d8658ddbc2f0b0439b20e07f4281d15da823"
    end
    on_arm do
      url "https://github.com/gerukin/editio/releases/download/v0.1.0/editio-0.1.0-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "6f01d7a25e6324cb3001bf978f8c69eb182fa5c9d9a6693e63e2b239d5a8bc33"
    end
  end

  on_macos do
    on_intel do
      url "https://github.com/gerukin/editio/releases/download/v0.1.0/editio-0.1.0-x86_64-apple-darwin.tar.gz"
      sha256 "44d90f444f1fe63865ed193cf3bdaa37b9dedaf472684b18e05a9c5ffa6cf5ea"
    end
    on_arm do
      url "https://github.com/gerukin/editio/releases/download/v0.1.0/editio-0.1.0-aarch64-apple-darwin.tar.gz"
      sha256 "30072e2431fee0ee70e3bffe9b9ed585f9c361c17aa42a9462f0b8fda4ca9865"
    end
  end

  def install
    bin.install "editio"
    doc.install "LICENSE", "LICENSE-tapp-ui", "THIRD_PARTY.html", "RELEASE.txt"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/editio --version")
  end
end
