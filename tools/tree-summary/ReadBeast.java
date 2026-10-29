import java.io.File;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import beast.base.evolution.tree.Node;
import beast.base.parser.NexusParser;

/** Independent import check through BEAST.base's NEXUS parser. */
public class ReadBeast {
    static List<String> visit(Node node) {
        List<String> taxa = new ArrayList<>();
        if (node.isLeaf()) taxa.add(node.getID());
        else for (Node child : node.getChildren()) taxa.addAll(visit(child));
        Collections.sort(taxa);
        System.out.println(String.join(",", taxa) + "\t" + node.getHeight() + "\t"
            + node.getMetaData("posterior") + "\t" + node.getMetaData("rate"));
        if (!node.isRoot() && node.getLength() < 0) throw new IllegalStateException("Negative branch");
        return taxa;
    }
    public static void main(String[] args) throws Exception {
        NexusParser parser = new NexusParser();
        parser.parseFile(new File(args[0]));
        if (parser.trees.size() != 1) throw new IllegalStateException("Expected one summary tree");
        visit(parser.trees.get(0).getRoot());
    }
}
