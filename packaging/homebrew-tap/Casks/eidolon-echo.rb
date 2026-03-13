cask "eidolon-echo" do
  version :latest
  sha256 :no_check

  url "https://github.com/HughLFree/eidolon-echo/releases/latest/download/Eidolon-Echo.dmg",
      verified: "github.com/HughLFree/eidolon-echo/"
  name "Eidolon-Echo"
  desc "Desktop AI companion with roleplay and local memory"
  homepage "https://github.com/HughLFree/eidolon-echo"

  app "Eidolon-Echo.app"

  uninstall quit: "io.github.hughlfree.eidolonecho"

  # Intentionally includes Application Support so `--zap` performs a privacy-first
  # uninstall for this third-party tap, including local chats and stored API keys.
  zap trash: [
    "~/Library/Application Support/io.github.hughlfree.eidolonecho",
    "~/Library/Caches/io.github.hughlfree.eidolonecho",
    "~/Library/Logs/io.github.hughlfree.eidolonecho",
    "~/Library/Preferences/io.github.hughlfree.eidolonecho.plist",
  ]

  caveats do
    <<~EOS
      `brew uninstall --cask --zap eidolon-echo` will also remove local chat history,
      provider settings, and stored API keys from this Mac.
    EOS
  end
end
