# Song-to-Video Quickstart

Every Magic Studio song project starts from an audio recording so word timing drives the storyboard, character lip-sync, kinetic lyrics, and master timeline.

## 1. Create a Song Project

```sh
magic-cli new   --root ~/Videos/Magic   --name "Rendered Yesterday"   --audio ~/Music/rendered-yesterday.wav   --style "Stop-motion workshop puppets, warm amber stage lighting, deep teal shadows"   --world "Candlelit repair-bench theatre with velvet curtains"   --character "Friendly vintage CRT television puppet with expressive screen face"   --motion "Hand-drawn chalk star and wave accents synced to chorus verbs"
```

This writes `Rendered Yesterday.magic`, `timing-reference.json`, `transcript.srt`, `storyboard.designcraft`, and `film/rendered-yesterday.fcproj`.

## 2. Complete Storyboard Title Pages & Generate Scene Plates

```sh
magic-cli --project ~/Videos/Magic/rendered-yesterday/"Rendered Yesterday.magic" storyboard check
magic-cli --project ~/Videos/Magic/rendered-yesterday/"Rendered Yesterday.magic" storyboard bulk_generate
```

## 3. Cut Out, Rig & Lip-Sync Lead Character (`PhotoCraft` + `EffectCraft`)

```sh
PROJ=~/Videos/Magic/rendered-yesterday/"Rendered Yesterday.magic"
magic-cli --project "$PROJ" --save exec character.import path=assets/crt.png
magic-cli --project "$PROJ" --save exec character.cut character=crt remove_background=true rig=true
magic-cli --project "$PROJ" --save exec character.puppet.rig_face character=crt
magic-cli --project "$PROJ" --save exec character.puppet.lipsync character=crt
```

## 4. Publish Kinetic Lyrics (`V3`), Action Motion (`V4`) & Assemble `FilmCraft`

```sh
magic-cli --project "$PROJ" --save exec words.letter render=true studio=true
magic-cli --project "$PROJ" --save exec motion.publish render=true
magic-cli --project "$PROJ" --save exec video.assemble export=true
```
