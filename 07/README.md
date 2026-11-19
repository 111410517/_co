# Hack VM 翻譯器

在專案根目錄執行：

```powershell
rustc -O 07/vm_translator.rs -o vm07.exe
.\vm07.exe 07/StackArithmetic/SimpleAdd/SimpleAdd.vm
```

輸出是同目錄的 `SimpleAdd.asm`。第 7 章的測試腳本會設定堆疊指標。
