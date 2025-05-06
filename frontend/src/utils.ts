export default function convertWeiToBtc(wei: string): string {
    const ethInWei = BigInt(wei);
    const oneEthInWei = BigInt(10 ** 18); // 1 ETH = 10^18 Wei

    // Convert Wei to ETH (ETH = Wei / 10^18)
    const btcAmount = Number(ethInWei) / Number(oneEthInWei);

    // Format to 8 decimal places (standard BTC format)
    return btcAmount.toFixed(8);
}
