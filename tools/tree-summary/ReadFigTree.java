import java.io.FileReader;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import jebl.evolution.graphs.Node;
import jebl.evolution.io.NexusImporter;
import jebl.evolution.trees.RootedTree;

/** Exercise the importer distributed with FigTree, including its metadata parsing. */
public class ReadFigTree {
    static List<String> visit(RootedTree tree, Node node) {
        List<String> taxa = new ArrayList<>();
        if (tree.isExternal(node)) taxa.add(tree.getTaxon(node).getName());
        else for (Node child : tree.getChildren(node)) taxa.addAll(visit(tree, child));
        Collections.sort(taxa);
        System.out.println(String.join(",", taxa) + "\t" + tree.getHeight(node) + "\t"
            + node.getAttribute("posterior") + "\t" + node.getAttribute("rate"));
        if (tree.getLength(node) < 0) throw new IllegalStateException("Negative branch");
        return taxa;
    }
    public static void main(String[] args) throws Exception {
        try (FileReader file = new FileReader(args[0])) {
            NexusImporter importer = new NexusImporter(file);
            RootedTree tree = (RootedTree) importer.importNextTree();
            if (tree.conceptuallyUnrooted()) throw new IllegalStateException("Unrooted");
            visit(tree, tree.getRootNode());
            if (importer.hasTree()) throw new IllegalStateException("Expected one summary tree");
        }
    }
}
