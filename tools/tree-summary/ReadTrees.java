import java.io.File;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import beast.base.evolution.tree.Node;
import beast.base.parser.NexusParser;

/** Independent read of sampled trees through BEAST.base's NEXUS parser. */
public class ReadTrees {
    static List<String> visit(int index, Node node) {
        List<String> taxa = new ArrayList<>();
        if (node.isLeaf()) taxa.add(node.getID());
        else for (Node child : node.getChildren()) taxa.addAll(visit(index, child));
        Collections.sort(taxa);
        System.out.println(index + "\t" + String.join(",", taxa) + "\t" + node.getHeight() + "\t"
            + node.getMetaData("height") + "\t" + node.getMetaData("rate"));
        if (!node.isRoot() && node.getLength() < 0) throw new IllegalStateException("Negative branch");
        return taxa;
    }
    public static void main(String[] args) throws Exception {
        NexusParser parser = new NexusParser();
        parser.parseFile(new File(args[0]));
        if (parser.trees.isEmpty()) throw new IllegalStateException("No trees");
        for (int index = 0; index < parser.trees.size(); index++) visit(index, parser.trees.get(index).getRoot());
        System.err.println(parser.trees.size() + " trees");
    }
}
