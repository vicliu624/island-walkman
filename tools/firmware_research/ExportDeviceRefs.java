// Read-only references; no imported target code is executed.
// @category Island.FirmwareResearch
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.symbol.Reference;
import java.nio.file.*;
import java.nio.charset.StandardCharsets;

public class ExportDeviceRefs extends GhidraScript {
    @Override public void run() throws Exception {
        String[] args = getScriptArgs();
        Path output = Paths.get(args[0]);
        StringBuilder text = new StringBuilder("Static references; not target execution.\n");
        for (int i = 1; i < args.length; i++) {
            Address target = toAddr(args[i]);
            text.append("TARGET ").append(target).append("\n");
            for (Reference ref : getReferencesTo(target)) {
                Function function = getFunctionContaining(ref.getFromAddress());
                text.append(ref.getFromAddress()).append(" ").append(ref.getReferenceType());
                if (function != null) text.append(" IN ").append(function.getName()).append(" ENTRY ").append(function.getEntryPoint());
                text.append("\n");
            }
        }
        if (Files.exists(output)) throw new Exception("Refusing overwrite " + output);
        Files.writeString(output, text, StandardCharsets.UTF_8);
        println(text.toString());
    }
}
