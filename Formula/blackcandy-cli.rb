class BlackcandyCli < Formula
  desc "Command-line client for Black Candy music servers"
  homepage "https://github.com/blackcandy-org/cli"
  url "https://github.com/blackcandy-org/cli/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "752ad4ecb9bad6412730edeec167b0393596e8e2eeef3fde2b02790714e4d5da"
  license "MIT"
  head "https://github.com/blackcandy-org/cli.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match "A command-line client for Black Candy", shell_output("#{bin}/blackcandy --help")
  end

  def caveats
    <<~EOS
      Playback uses mpv by default. Install it with:
        brew install mpv
    EOS
  end
end
