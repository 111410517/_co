# Hack VM 翻譯器

第 8 章的 Rust 程式，可以把 `.vm` 翻譯成 Hack 組合語言。也支援第 7 章的堆疊運算和記憶體存取。

在專案根目錄執行：

```powershell
rustc -O 08/vm_translator.rs -o vm_translator.exe
.\vm_translator.exe 08/FunctionCalls/SimpleFunction/SimpleFunction.vm
.\vm_translator.exe 08/FunctionCalls/FibonacciElement --bootstrap
```

輸入單一檔案會產生同名的 `.asm`。輸入資料夾會翻譯裡面的 `.vm`，合成一個以資料夾命名的 `.asm`。加上 `--bootstrap` 會把 `SP` 設成 256，再呼叫 `Sys.init`。

測試時用 Hack CPU Emulator 跑第 8 章附的六份測試。BasicLoop、FibonacciSeries、SimpleFunction、NestedCall、FibonacciElement、StaticsTest 的輸出都和 `.cmp` 一致。FibonacciElement 和 StaticsTest 要加 `--bootstrap`；NestedCall 的測試會自己設定堆疊。
