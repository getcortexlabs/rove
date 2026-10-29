package rove.reference;

import beastfx.app.tools.LogAnalyser;
import java.nio.file.Files;
import java.nio.file.Path;
import org.json.JSONObject;

/** Test-only reference process; never linked into Rove. */
public final class TraceReference {
    private static Object finite(double value) {
        return Double.isFinite(value) ? value : JSONObject.NULL;
    }
    public static void main(String[] args) throws Exception {
        LogAnalyser analyser = new LogAnalyser(args[0], Integer.parseInt(args[1]), true);
        JSONObject columns = new JSONObject();
        for (String label : analyser.getLabels()) {
            columns.put(label, new JSONObject()
                .put("mean", finite(analyser.getMean(label)))
                .put("sd", finite(analyser.getStdDev(label)))
                .put("ess", finite(analyser.getESS(label)))
                .put("hpd_low", finite(analyser.get95HPDlow(label)))
                .put("hpd_high", finite(analyser.get95HPDup(label))));
        }
        Files.writeString(Path.of(args[2]), columns.toString(2) + "\n");
    }
}
