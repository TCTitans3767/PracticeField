export async function GET(request: Request) {
    return Response.json(
        {
            isConnected: true,
            timestamp: new Date().toISOString(),
        },
        { status: 200 }
    );
}