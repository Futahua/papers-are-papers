Add-Type -AssemblyName System.Drawing

$size = 256
$bitmap = [System.Drawing.Bitmap]::new($size, $size)
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
$graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$graphics.Clear([System.Drawing.ColorTranslator]::FromHtml("#F1EEE6"))

$ink = [System.Drawing.ColorTranslator]::FromHtml("#191917")
$paper = [System.Drawing.ColorTranslator]::FromHtml("#FAF8F2")
$pen = [System.Drawing.Pen]::new($ink, 12)
$pen.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
$pen.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
$brush = [System.Drawing.SolidBrush]::new($paper)

$graphics.FillEllipse($brush, 44, 61, 168, 168)
$graphics.DrawEllipse($pen, 44, 61, 168, 168)
$graphics.DrawArc($pen, 88, 26, 80, 82, 178, 184)
$graphics.FillRectangle($brush, 70, 139, 116, 67)
$graphics.DrawRectangle($pen, 70, 139, 116, 67)
$graphics.DrawLine($pen, 97, 140, 97, 160)
$graphics.DrawLine($pen, 159, 140, 159, 160)

$iconsDirectory = Join-Path $PSScriptRoot "..\src-tauri\icons"
New-Item -ItemType Directory -Force -Path $iconsDirectory | Out-Null

$pngPath = Join-Path $iconsDirectory "icon.png"
$icoPath = Join-Path $iconsDirectory "icon.ico"
$bitmap.Save($pngPath, [System.Drawing.Imaging.ImageFormat]::Png)

$handle = $bitmap.GetHicon()
$icon = [System.Drawing.Icon]::FromHandle($handle)
$stream = [System.IO.File]::Create($icoPath)
$icon.Save($stream)
$stream.Close()

$icon.Dispose()
$pen.Dispose()
$brush.Dispose()
$graphics.Dispose()
$bitmap.Dispose()

Write-Output $icoPath
