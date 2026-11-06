// Export the decompiled C of every function to one greppable file.
// Usage (headless): -postScript ExportDecompiled.java <output.c>
// @category yo3
import java.io.FileWriter;
import java.io.PrintWriter;

import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.listing.Function;

public class ExportDecompiled extends GhidraScript {
    @Override
    public void run() throws Exception {
        String out = getScriptArgs().length > 0 ? getScriptArgs()[0] : "decompiled.c";
        DecompInterface ifc = new DecompInterface();
        ifc.openProgram(currentProgram);
        int n = 0, failed = 0;
        try (PrintWriter w = new PrintWriter(new FileWriter(out))) {
            for (Function f : currentProgram.getFunctionManager().getFunctions(true)) {
                if (monitor.isCancelled()) break;
                DecompileResults r = ifc.decompileFunction(f, 60, monitor);
                w.printf("// ===== %s @ %s%n", f.getName(true), f.getEntryPoint());
                if (r != null && r.decompileCompleted()) {
                    w.println(r.getDecompiledFunction().getC());
                } else {
                    w.println("// decompilation failed");
                    failed++;
                }
                if (++n % 1000 == 0) println("decompiled " + n);
            }
        }
        println("done: " + n + " functions, " + failed + " failed -> " + out);
    }
}
