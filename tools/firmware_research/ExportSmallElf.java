// Static decompilation only. Never execute imported target code.
// @category Island.FirmwareResearch
import ghidra.app.script.GhidraScript;
import ghidra.app.decompiler.*;
import ghidra.program.model.listing.*;
import java.nio.file.*;

public class ExportSmallElf extends GhidraScript {
    public void run() throws Exception {
        Path output = Paths.get(getScriptArgs()[0]).resolve(currentProgram.getName());
        Files.createDirectories(output);
        String[] args = getScriptArgs();
        if (args.length > 1) {
            var entry = toAddr(args[1]);
            if (getFunctionAt(entry) == null) {
                disassemble(entry);
                createFunction(entry, "research_entry");
            }
        }
        DecompInterface decompiler = new DecompInterface();
        decompiler.openProgram(currentProgram);
        try {
            FunctionIterator functions = currentProgram.getFunctionManager().getFunctions(true);
            int count = 0;
            while (functions.hasNext()) {
                Function function = functions.next();
                if (args.length > 1 && !function.getEntryPoint().equals(toAddr(args[1]))) continue;
                if (function.isExternal() || function.isThunk()) continue;
                if (++count > 150) throw new Exception("Small ELF function bound exceeded");
                DecompileResults result = decompiler.decompileFunction(function, 30, monitor);
                String text = "// Static analysis only: " + currentProgram.getName() + "\n// Entry: " + function.getEntryPoint() + "\n";
                if (result.decompileCompleted() && result.getDecompiledFunction() != null)
                    text += result.getDecompiledFunction().getC();
                else text += "// INCOMPLETE: " + result.getErrorMessage();
                Path file = output.resolve(function.getEntryPoint() + ".c.txt");
                if (Files.exists(file)) throw new Exception("Refusing overwrite: " + file);
                Files.writeString(file, text);
            }
            println("Static functions exported: " + count);
        } finally { decompiler.dispose(); }
    }
}
