import SwiftUI

struct ContentView: View {
    @State private var rustMessage: String = ""
    
    var body: some View {
        VStack {
            Image(systemName: "gear")
                .imageScale(.large)
                .foregroundStyle(.tint)
            Text(rustMessage)
                .font(.title)
                .padding()
            Button("Get Message from Rust") {
                getRustMessage()
            }
            .buttonStyle(.borderedProminent)
        }
        .padding()
        .onAppear {
            getRustMessage()
        }
    }
    
    private func getRustMessage() {
        let cString = hello_rust_world()
        rustMessage = String(cString: cString!)
        free_string(cString)
    }
}