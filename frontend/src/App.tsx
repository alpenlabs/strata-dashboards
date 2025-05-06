import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { Route, BrowserRouter as Router, Routes } from "react-router-dom";
import Dashboard from "./pages/Dashboard";

const queryClient = new QueryClient();
export default function App() {
    return (
        <QueryClientProvider client={queryClient}>
            <Router>
                <Routes>
                    <Route path="/*" element={<Dashboard />} />{" "}
                    {/* Everything inside Dashboard */}
                </Routes>
            </Router>
        </QueryClientProvider>
    );
}
