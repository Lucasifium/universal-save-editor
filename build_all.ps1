# Builds SaveEditorOffline for every platform the machine can build, into dist\.
#   Windows x64 + ARM64 (MSVC), Linux x64 + ARM64 (cargo-zigbuild), Android APK (arm64 + x86_64).
# macOS is built by GitHub Actions (.github/workflows/build.yml), since Apple's SDK only runs on Macs.
#
#   .\build_all.ps1              everything
#   .\build_all.ps1 -Only android,linux-x64

param([string[]]$Only = @())

$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot

# Android SDK: ANDROID_HOME, else Android Studio's usual place, else an android-sdk folder beside this one.
$SdkRoot = @($env:ANDROID_HOME, $env:ANDROID_SDK_ROOT, "$env:LOCALAPPDATA\Android\Sdk", (Join-Path $PSScriptRoot '..\android-sdk')) |
    Where-Object { $_ -and (Test-Path (Join-Path $_ 'ndk')) } | Select-Object -First 1
$Ndk = Get-ChildItem "$SdkRoot\ndk" -Directory | Sort-Object Name | Select-Object -Last 1
$NdkBin = "$($Ndk.FullName)\toolchains\llvm\prebuilt\windows-x86_64\bin"
$BuildTools = (Get-ChildItem "$SdkRoot\build-tools" -Directory | Sort-Object Name | Select-Object -Last 1).FullName
$Platform = (Get-ChildItem "$SdkRoot\platforms" -Directory | Sort-Object Name | Select-Object -Last 1).FullName
$Jdk = (Get-ChildItem 'C:\Program Files\Microsoft' -Directory -Filter 'jdk-*' | Sort-Object Name | Select-Object -Last 1).FullName
$Version = (Select-String -Path Cargo.toml -Pattern '^version = "(.+)"').Matches[0].Groups[1].Value
$Name = 'SaveEditorOffline'
$env:CARGO_ZIGBUILD_ZIG_PATH = 'python -m ziglang'
$env:RUSTFLAGS = '-A linker_messages'

function Want($what) { $Only.Count -eq 0 -or $Only -contains $what }
function Run($exe) {
    & $exe @args
    if ($LASTEXITCODE -ne 0) { throw "$exe $args failed ($LASTEXITCODE)" }
}

New-Item -ItemType Directory -Force dist | Out-Null
$built = @()

if (Want 'windows-x64') {
    Write-Host '== Windows x64' -ForegroundColor Cyan
    Run cargo build --release --target x86_64-pc-windows-msvc
    Copy-Item "target\x86_64-pc-windows-msvc\release\$Name.exe" "dist\$Name-windows-x64.exe"
    $built += "dist\$Name-windows-x64.exe"
}
if (Want 'windows-arm64') {
    Write-Host '== Windows ARM64' -ForegroundColor Cyan
    Run cargo build --release --target aarch64-pc-windows-msvc
    Copy-Item "target\aarch64-pc-windows-msvc\release\$Name.exe" "dist\$Name-windows-arm64.exe"
    $built += "dist\$Name-windows-arm64.exe"
}
foreach ($arch in @(@('x64', 'x86_64'), @('arm64', 'aarch64'))) {
    if (-not (Want "linux-$($arch[0])")) { continue }
    Write-Host "== Linux $($arch[0])" -ForegroundColor Cyan
    # .2.28: works on any Linux with glibc 2.28+ (2018 onwards).
    Run cargo zigbuild --release --target "$($arch[1])-unknown-linux-gnu.2.28"
    Run python -I packaging\package.py tar "target\$($arch[1])-unknown-linux-gnu\release\$Name" "dist\$Name-linux-$($arch[0]).tar.gz"
    $built += "dist\$Name-linux-$($arch[0]).tar.gz"
}

if (Want 'android') {
    Write-Host '== Android APK' -ForegroundColor Cyan
    $env:JAVA_HOME = $Jdk
    $env:PATH = "$Jdk\bin;$env:PATH"
    $libs = @()
    foreach ($abi in @(@('arm64-v8a', 'aarch64-linux-android'), @('x86_64', 'x86_64-linux-android'))) {
        $t = $abi[1]; $u = $t.Replace('-', '_'); $up = $u.ToUpper()
        Set-Item "env:CC_$u" "$NdkBin\$($t)26-clang.cmd"
        Set-Item "env:AR_$u" "$NdkBin\llvm-ar.exe"
        Set-Item "env:CARGO_TARGET_$($up)_LINKER" "$NdkBin\$($t)26-clang.cmd"
        # 16 KB pages: required for Android 15+ devices that use them.
        $env:RUSTFLAGS = '-A linker_messages -C link-arg=-Wl,-z,max-page-size=16384'
        Run cargo rustc --release --lib --crate-type cdylib --target $t
        $so = "target\$t\release\libsave_editor_offline.so"
        Run "$NdkBin\llvm-strip.exe" --strip-unneeded $so -o "target\$t\release\libsave_editor_offline.stripped.so"
        $stage = "target\apk\lib-$($abi[0])"
        New-Item -ItemType Directory -Force $stage | Out-Null
        Copy-Item "target\$t\release\libsave_editor_offline.stripped.so" "$stage\libsave_editor_offline.so"
        $libs += "lib/$($abi[0])/libsave_editor_offline.so=$stage\libsave_editor_offline.so"
    }
    $env:RUSTFLAGS = '-A linker_messages'

    $apkDir = 'target\apk'
    Remove-Item -Recurse -Force "$apkDir\res", "$apkDir\compiled" -ErrorAction SilentlyContinue
    # Launcher icons: PNGs written by build.rs.
    $icons = Get-ChildItem 'target\aarch64-linux-android\release\build' -Directory -Filter 'save_editor_offline-*' |
        ForEach-Object { "$($_.FullName)\out\icons" } | Where-Object { Test-Path "$_\192.png" } | Select-Object -First 1
    Run python -I packaging\package.py icons $icons "$apkDir\res"
    $p = $Version.Split('.')
    $code = [int]$p[0] * 10000 + [int]$p[1] * 100 + [int]$p[2]
    (Get-Content packaging\android\AndroidManifest.xml -Raw).Replace('@VERSION@', $Version).Replace('@VERSION_CODE@', "$code") |
        Set-Content "$apkDir\AndroidManifest.xml" -Encoding utf8
    New-Item -ItemType Directory -Force "$apkDir\compiled" | Out-Null
    Run "$BuildTools\aapt2.exe" compile --dir "$apkDir\res" -o "$apkDir\compiled"
    $flats = @(Get-ChildItem "$apkDir\compiled" -Filter *.flat | ForEach-Object FullName)
    Run "$BuildTools\aapt2.exe" link -o "$apkDir\unsigned.apk" --manifest "$apkDir\AndroidManifest.xml" -I "$Platform\android.jar" @flats
    # MainActivity.java: on-screen keyboard support.
    $jout = "$apkDir\java"
    Remove-Item -Recurse -Force $jout -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force "$jout\classes" | Out-Null
    $sources = @(Get-ChildItem packaging\android\java -Recurse -Filter *.java | ForEach-Object FullName)
    Run "$Jdk\bin\javac.exe" -source 8 -target 8 '-Xlint:-options' -classpath "$Platform\android.jar" -d "$jout\classes" @sources
    $classes = @(Get-ChildItem "$jout\classes" -Recurse -Filter *.class | ForEach-Object FullName)
    Run "$BuildTools\d8.bat" --release --min-api 26 --lib "$Platform\android.jar" --output $jout @classes
    Run python -I packaging\package.py add "$apkDir\unsigned.apk" "classes.dex=$jout\classes.dex" @libs
    Run "$BuildTools\zipalign.exe" -f -p 4 "$apkDir\unsigned.apk" "$apkDir\aligned.apk"

    # Signing key: made once and kept in packaging\android. Keep it safe; Android only installs
    # updates signed with the same key.
    $ks = 'packaging\android\release.keystore'
    $pwFile = 'packaging\android\keystore-password.txt'
    if (-not (Test-Path $ks)) {
        $pw = -join ((48..57) + (65..90) + (97..122) | Get-Random -Count 24 | ForEach-Object { [char]$_ })
        Set-Content $pwFile $pw -NoNewline -Encoding ascii
        Run "$Jdk\bin\keytool.exe" -genkeypair -keystore $ks -alias saveeditoroffline -keyalg RSA -keysize 4096 -validity 10000 `
            -storepass $pw -keypass $pw -dname 'CN=SaveEditorOffline'
    }
    Run "$BuildTools\apksigner.bat" sign --ks $ks --ks-pass "file:$pwFile" --out "dist\$Name-android.apk" "$apkDir\aligned.apk"
    Run "$BuildTools\apksigner.bat" verify "dist\$Name-android.apk"
    Remove-Item "dist\$Name-android.apk.idsig" -ErrorAction SilentlyContinue
    $built += "dist\$Name-android.apk"
}

Write-Host ''
Write-Host "Built $Name $Version" -ForegroundColor Green
foreach ($f in $built) { '{0,-45} {1,8:N1} MB' -f $f, ((Get-Item $f).Length / 1MB) }
Write-Host 'macOS: start "macOS build" by hand in the GitHub Actions tab when needed (it never runs on its own).'
