#!/bin/bash
# W10 part 1 (no sign-in): load each site with the Safari identity and the stock WebKit UA,
# record title / final URL / text start / playing media, screenshot each.
# usage: ./compat.sh  -> results/w10/compat.jsonl + results/w10/<site>-<ua>.png
cd "$(dirname "$0")"
mkdir -p results/w10
out=results/w10/compat.jsonl
: > "$out"
sites=(
  "google-signin https://accounts.google.com/"
  "gmail https://mail.google.com/"
  "youtube https://www.youtube.com/watch?v=aqz-KE-bpKQ"
  "maps https://www.google.com/maps/@28.6139,77.209,13z"
  "meet https://meet.google.com/"
  "chatgpt https://chatgpt.com/"
  "microsoft https://login.microsoftonline.com/"
  "outlook https://outlook.live.com/mail/"
  "teams https://teams.microsoft.com/"
  "whatsapp https://web.whatsapp.com/"
  "discord https://discord.com/login"
  "github https://github.com/login"
  "slack https://app.slack.com/"
  "figma https://www.figma.com/login"
  "hdfc https://netbanking.hdfcbank.com/netbanking/"
  "sbi https://retail.onlinesbi.sbi/retail/login.htm"
  "reddit https://www.reddit.com/"
  "x https://x.com/login"
)
for s in "${sites[@]}"; do
  name=${s%% *}; url=${s#* }
  for ua in safari webkit; do
    flag=; [ "$ua" = safari ] && flag=--safari
    res=$(W10_SHOT=results/w10/$name-$ua.png timeout 60 target/release/w9-baseline --compat $flag "$url" 2>/dev/null | grep '^{')
    echo "{\"site\":\"$name\",\"ua\":\"$ua\",\"res\":${res:-null}}" | tee -a "$out" | cut -c1-200
  done
done
