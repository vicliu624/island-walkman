// Static analysis only: never invokes the imported program or any Sony method.
// @category Island.FirmwareResearch
import ghidra.app.script.GhidraScript;
import ghidra.app.decompiler.*;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.symbol.Reference;
import java.nio.file.*;
import java.nio.charset.StandardCharsets;

public class ExportFlow extends GhidraScript {
    @Override public void run() throws Exception {
        String[] args = getScriptArgs();
        Path output = Paths.get(args[0]);
        Files.createDirectories(output);
        DecompInterface decompiler = new DecompInterface();
        decompiler.openProgram(currentProgram);
        try {
            for (int i = 1; i < args.length; i++) {
                Address address = toAddr(args[i]);
                Function function = getFunctionAt(address);
                if (function == null) function = getFunctionContaining(address);
                if (function == null) {
                    println("No recovered function at " + address);
                    continue;
                }
                DecompileResults result = decompiler.decompileFunction(function, 45, monitor);
                StringBuilder text = new StringBuilder();
                text.append("// Static Ghidra export: ").append(currentProgram.getName()).append("\n");
                text.append("// Requested: ").append(address).append("; entry: ").append(function.getEntryPoint()).append("\n");
                text.append("// Recovered body: ").append(function.getBody()).append("\n");
                for (Reference ref : getReferencesTo(function.getEntryPoint())) {
                    text.append("// Incoming: ").append(ref.getFromAddress()).append(" ").append(ref.getReferenceType()).append("\n");
                }
                for (Function called : function.getCalledFunctions(monitor)) {
                    text.append("// Calls: ").append(called.getEntryPoint()).append(" ").append(called.getName()).append("\n");
                }
                if (result.decompileCompleted() && result.getDecompiledFunction() != null) {
                    text.append(result.getDecompiledFunction().getC());
                } else {
                    text.append("// Decompile failure: ").append(result.getErrorMessage());
                }
                Files.writeString(output.resolve(args[i] + ".c"), text, StandardCharsets.UTF_8);
                println("Exported " + address + " entry=" + function.getEntryPoint());
            }
        } finally { decompiler.dispose(); }
    }
}
