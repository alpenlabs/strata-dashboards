import { useQuery } from "@tanstack/react-query";

export type Account = {
    address: string,
    deployed_at: string,
    gas_used: string,
}

export type UsageStats = {
    user_ops_count: Map<string, number>,
    total_gas_used: Map<string, number>,
    unique_active_accounts: Map<string, number>,
    recent_accounts: Array<Account>,
    top_gas_consumers: Array<Account>,
};

const API_BASE_URL = import.meta.env.API_BASE_URL || "http://localhost:3000";
const fetchUsageStats = async (): Promise<UsageStats> => {
    const response = await fetch(`${API_BASE_URL}/api/usage_stats`);
    if (!response.ok) {
        throw new Error("Failed to fetch usage stats");
    }
    return response.json();
};

export const useUsageStats = () => {
    return useQuery({
        queryKey: ["usageStats"],
        queryFn: fetchUsageStats,
        refetchInterval: 10000, // ✅ Auto-refetch every 30s
    });
};
