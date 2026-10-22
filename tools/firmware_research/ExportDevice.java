// Static ARM analysis only; imported programs and Sony methods are never run.
// @category Island.FirmwareResearch
import ghidra.app.script.GhidraScript;
import ghidra.app.decompiler.*;
import ghidra.program.model.listing.*;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.address.Address;
import java.nio.file.*;
import java.nio.charset.StandardCharsets;

public class ExportDevice extends GhidraScript {
    private void export(Function function, Path directory) throws Exception {
        DecompInterface decompiler = new DecompInterface();
        decompiler.openProgram(currentProgram);
        try {
            StringBuilder text = new StringBuilder("// Static analysis, not target execution.\n");
            text.append("// Input: ").append(currentProgram.getName()).append("\n");
            text.append("// Language: ").append(currentProgram.getLanguageID()).append("\n");
            text.append("// Entry: ").append(function.getEntryPoint()).append("; body: ").append(function.getBody()).append("\n");
            for (Reference reference : getReferencesTo(function.getEntryPoint()))
                text.append("// Incoming: ").append(reference.getFromAddress()).append(" ").append(reference.getReferenceType()).append("\n");
            for (Function called : function.getCalledFunctions(monitor))
                text.append("// Calls: ").append(called.getEntryPoint()).append(" ").append(called.getName()).append("\n");
            DecompileResults result = decompiler.decompileFunction(function, 45, monitor);
            if (result.decompileCompleted() && result.getDecompiledFunction() != null)
                text.append(result.getDecompiledFunction().getC());
            else text.append("// Incomplete: ").append(result.getErrorMessage());
            String filename = function.getEntryPoint() + "-" + function.getName().replaceAll("[^a-zA-Z0-9_.-]", "_") + ".c";
            Path output = directory.resolve(filename);
            if (Files.exists(output)) throw new Exception("Refusing to overwrite " + output);
            Files.writeString(output, text.toString(), StandardCharsets.UTF_8);
            println("Exported " + function.getName() + " at " + function.getEntryPoint());
        } finally { decompiler.dispose(); }
    }

    @Override public void run() throws Exception {
        String[] args = getScriptArgs();
        Path output = Paths.get(args[0]);
        Files.createDirectories(output);
        for (int i = 1; i < args.length; i++) {
            Function selected = null;
            if (args[i].startsWith("0x")) {
                selected = getFunctionAt(toAddr(args[i]));
                if (selected == null) selected = getFunctionContaining(toAddr(args[i]));
            } else {
                FunctionIterator iterator = currentProgram.getFunctionManager().getFunctions(true);
                while (iterator.hasNext()) {
                    Function function = iterator.next();
                    if (function.getName().equals(args[i])) { selected = function; break; }
                }
            }
            if (selected == null) println("NOT FOUND: " + args[i]);
            else export(selected, output);
        }
    }
}
