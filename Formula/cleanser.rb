# Homebrew Formula for Cleanser
# To use this formula:
# 1. Create a GitHub release with a tarball of the source
# 2. Update the url and sha256 below
# 3. Either submit to homebrew-core or create your own tap

class Cleanser < Formula
  desc "Fast CLI tool for clearing macOS storage space"
  homepage "https://github.com/phpfc/cleanser"
  url "https://github.com/phpfc/cleanser/archive/refs/tags/v0.1.1.tar.gz"
  sha256 "53cb7e8eada64b84eb7c73d25a5407e4add3069e8b2e5aeae36a9b726c4cd075"
  license "MIT"
  head "https://github.com/phpfc/cleanser.git", branch: "main"

  bottle do
    root_url "https://github.com/phpfc/cleanser/releases/download/v0.1.1"
    rebuild 1
    sha256 cellar: :any_skip_relocation, arm64_tahoe: "ca242d4d530414ae8a8b941036badd1531b42a18ae29010577566dd86976e4f2"
  end

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    system "#{bin}/cleanser", "--version"
    assert_match "cleanser 0.1.0", shell_output("#{bin}/cleanser --version")
  end
end
