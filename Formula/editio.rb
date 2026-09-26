class Editio < Formula
  desc "Fast, minimal terminal text viewer and editor"
  homepage "https://github.com/gerukin/editio"
  version "0.1.1"
  license "MIT"

  on_linux do
    on_intel do
      url "https://github.com/gerukin/editio/releases/download/v0.1.1/editio-0.1.1-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "74d40def5ab778fe8b0704fac58cd00fd0a369ddfe83e6440c32b4670262b4b8"
    end
    on_arm do
      url "https://github.com/gerukin/editio/releases/download/v0.1.1/editio-0.1.1-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "cc1a8ee7b8ae60f08d51a6c1e977db2bf5547827239f396144ad367e9db8b3de"
    end
  end

  on_macos do
    on_intel do
      url "https://github.com/gerukin/editio/releases/download/v0.1.1/editio-0.1.1-x86_64-apple-darwin.tar.gz"
      sha256 "53333696d0a132613505bb8d5af4b53533f850fb3225ccfda690469a623f3205"
    end
    on_arm do
      url "https://github.com/gerukin/editio/releases/download/v0.1.1/editio-0.1.1-aarch64-apple-darwin.tar.gz"
      sha256 "a0894232fb571a1f5ed0c716e55769c6cc1e37fc8c28bdfedb5625f5b9f2f7a2"
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
