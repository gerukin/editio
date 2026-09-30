class Editio < Formula
  desc "Fast, minimal terminal text viewer and editor"
  homepage "https://github.com/gerukin/editio"
  version "0.1.2"
  license "MIT"

  on_linux do
    on_intel do
      url "https://github.com/gerukin/editio/releases/download/v0.1.2/editio-0.1.2-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "dde1bea4c0768ed855a341dd42f02acf47ba54d53ec501b883a078e4bb9b5ce0"
    end
    on_arm do
      url "https://github.com/gerukin/editio/releases/download/v0.1.2/editio-0.1.2-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "bf28fbc9710506f19e9112934a29a58075333114cf3bb8d1826ef64517ed060d"
    end
  end

  on_macos do
    on_intel do
      url "https://github.com/gerukin/editio/releases/download/v0.1.2/editio-0.1.2-x86_64-apple-darwin.tar.gz"
      sha256 "fe339a10795d4bcdd55c212cc8a9bd60fdf73732a7f66436fafb359d37fefa7e"
    end
    on_arm do
      url "https://github.com/gerukin/editio/releases/download/v0.1.2/editio-0.1.2-aarch64-apple-darwin.tar.gz"
      sha256 "73bd94081a406cbc7b3cf6ff4e7514ff61c2f4f55ed993d11acc320a86137ed0"
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
