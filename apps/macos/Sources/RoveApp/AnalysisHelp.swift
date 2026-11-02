import SwiftUI

enum AnalysisHelpTopic {
  case files, prediction

  var title: String {
    self == .files ? "A guide to your run folder" : "What does a predictive check tell me?"
  }
  var guide: String { self == .files ? "run-files" : "posterior-prediction" }
  var introduction: String {
    self == .files
      ? "Start with the trace, then summarize the trees. Keep the full run folder and the original matrix if you want to revisit or resume the analysis."
      : "If this model described the data well, could it produce matrices with features like the ones we observed? Rove draws trees and parameters from your run and uses them to simulate new matrices."
  }
  var sections: [(String, String)] {
    switch self {
    case .files:
      [
        (
          "1. Check the sampling",
          "results/trace.log contains sampled numerical values. Open Diagnostics in Rove, or use Tracer. A finished run still needs sampling and tree-agreement checks."
        ),
        (
          "2. Summarize the trees",
          "results/trees.nexus contains many sampled trees, including the initial state. Choose Summarize trees and review burn-in. In summaries/<summary>/, mcc.tree is the annotated representative tree to explore in Rove or FigTree; samples.trees contains raw samples. Renaming .nexus to .trees does not summarize or convert a file."
        ),
        (
          "3. Keep the context",
          "setup.rove.json records the requested setup. session.json records the app session and status. results/run.json records what the engine ran. dating.json is present when fossil dating is enabled. These JSON records do not replace a checkpoint."
        ),
        (
          "4. Resume from a checkpoint",
          "results/checkpoint.json points to a checkpoint-<step>.bin file. Keep both, the trace, trees and run records. Resume also needs the unchanged original matrix and the same Rove build. Rove does not copy the matrix or engine into the run folder."
        ),
        (
          "5. Find logs and later reports",
          "events.jsonl records progress events; engine.stderr.log holds engine messages, not sampled statistics. Optional diagnostics/, summaries/, comparisons/ and predictions/ folders hold later reports. A predictive check includes predictive.json, replicates.jsonl, characters.tsv and simulated matrices/."
        ),
        (
          "Several chains",
          "Each chain has its own trace, trees, run record and checkpoints under results/chain-01/, chain-02/ and so on. The group record stays in results/run.json. Use each chain's outputs when checking sampling."
        ),
        (
          "What should I share?",
          "For a tree figure, share mcc.tree and the summary settings. To review sampling, include the trace and raw trees. To reproduce an analysis, keep the complete folder, original matrix and Rove version. Check file paths and sensitive data before sharing. Partial or interrupted files need review before interpretation."
        ),
      ]
    case .prediction:
      [
        (
          "1. Start with a sampled run",
          "Try the bundled example matrix. Finish an analysis, review Diagnostics and agreement across independent runs, then open its History entry and Posterior predictive check. The check uses the selected chain's posterior, not a new tree search."
        ),
        (
          "2. Choose burn-in and replicates",
          "Burn-in discards the beginning of the saved samples. With 10,001 saved states and 10% burn-in, 1,000 are discarded and 9,001 remain. Review the trace to choose a suitable value. Each replicate draws one retained state with replacement and simulates a matrix; 100 replicates means 100 simulated matrices, not 100 independent posterior samples."
        ),
        (
          "3. Compare one feature",
          "For an illustrative report, suppose the observed matrix has 30 informative characters. Of 100 valid simulations, 12 have at least 30, 96 have at most 30, and 8 have exactly 30. The upper tail is 0.12 and the lower tail is 0.96. Ties count in both, so the two tails need not add to one. These numbers are an example, not a result for the bundled matrix."
        ),
        (
          "4. Read the saved report",
          "The report lists each statistic, valid pairs, lower and upper tails, ties and simulation intervals. A small tail suggests a feature the model struggles to reproduce. Inspect that feature and the simulated matrices; there is no universal pass/fail threshold. Some statistics, such as parsimony length, compare both matrices on the same sampled tree."
        ),
        (
          "What this cannot tell you",
          "A tail fraction is not the probability that the model or tree is correct. This checks model adequacy, not convergence, clade support or which competing model is best. More replicates reduce simulation noise; they cannot repair poor posterior sampling. The displayed intervals cover simulation error only."
        ),
        (
          "Keep the check",
          "Open Check record for predictive.json, Replicate matrices for the simulated data, or Open folder for all outputs. The record preserves the source samples, burn-in and seed so the check can be inspected later."
        ),
      ]
    }
  }
}

struct AnalysisHelpButton: View {
  let topic: AnalysisHelpTopic
  let strings: AppStrings
  @State private var presented = false

  var body: some View {
    Button {
      presented = true
    } label: {
      Label(
        strings(topic == .files ? "What are these files?" : "Walk through an example"),
        systemImage: "questionmark.circle")
    }
    .buttonStyle(LabButtonStyle(compact: true))
    .sheet(isPresented: $presented) { AnalysisHelpView(topic: topic, strings: strings) }
  }
}

struct AnalysisHelpView: View {
  @Environment(\.dismiss) private var dismiss
  let topic: AnalysisHelpTopic
  let strings: AppStrings

  var body: some View {
    VStack(alignment: .leading, spacing: 0) {
      HStack(alignment: .top, spacing: 18) {
        Image(systemName: topic == .files ? "folder.badge.questionmark" : "square.on.square.dashed")
          .font(.system(size: 30, weight: .light)).foregroundStyle(Lab.lilac)
        Text(strings(topic.title)).font(Lab.display(27))
        Spacer()
        Button(strings("Close")) { dismiss() }.keyboardShortcut(.cancelAction)
          .buttonStyle(LabButtonStyle(compact: true))
      }.padding(28)
      Divider().overlay(Lab.border)
      ScrollView {
        VStack(alignment: .leading, spacing: 24) {
          Text(strings(topic.introduction)).font(.system(size: 14)).foregroundStyle(Lab.secondary)
          ForEach(topic.sections, id: \.0) { title, body in
            VStack(alignment: .leading, spacing: 9) {
              Text(strings(title)).font(.system(size: 14, weight: .semibold)).foregroundStyle(
                Lab.ice)
              Text(strings(body)).font(.system(size: 13)).foregroundStyle(Lab.secondary)
            }
          }
          Link(destination: strings.guideURL.appendingPathComponent(topic.guide)) {
            Label(strings("Read the full guide"), systemImage: "arrow.up.right")
          }.foregroundStyle(Lab.lilac)
        }.lineSpacing(5).textSelection(.enabled).padding(28).frame(
          maxWidth: .infinity, alignment: .leading)
      }
    }.frame(width: 700, height: 680).background(Lab.background).foregroundStyle(Lab.ink)
  }
}
